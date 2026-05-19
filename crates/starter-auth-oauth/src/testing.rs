//! Testing seams.
//!
//! Two helpers, both gated on `cfg(feature = "sqlite")` because the
//! cheap path to a working `UserStore` + `SessionStore` in v0.1 is
//! sqlite-in-memory (`starter-store-sqlite::testing::ephemeral`).
//!
//! - [`FakeProvider`] — implements [`crate::OAuthProvider`] without
//!   touching the network, returns a configurable
//!   [`crate::ProviderIdentity`], and records every `code` +
//!   `pkce_verifier` it saw so tests can assert
//!   "access-token-never-persists" by feeding a sentinel access
//!   token in the wrapping scenario.
//!
//! - [`MemoryEverything`] — one-call factory wiring
//!   `MemoryStateStore` + `SqliteIdentityStore` + the
//!   `SqliteUserStore` + `SqliteSessionStore` against an ephemeral
//!   pool, with the OAuth migrations already applied.

#![allow(missing_docs)]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::provider::{OAuthProvider, ProviderError, ProviderIdentity};

/// Deterministic, network-free [`OAuthProvider`] for tests.
///
/// The default identity is a verified GitHub-shaped one; tests
/// override either field by going through `FakeProvider::builder()`
/// or by mutating `*provider.identity.lock().unwrap()` between
/// requests.
pub struct FakeProvider {
    id: &'static str,
    /// What `fetch_identity` returns. Locked so a test can swap the
    /// identity between two sequential callback round trips against
    /// the same provider (used in the link-mode tests).
    pub identity: Mutex<Result<ProviderIdentity, ProviderError>>,
    /// Every `(code, pkce_verifier, redirect_uri)` triple the
    /// callback handler passed in. Tests assert against this to
    /// confirm the state-store-bound verifier round-tripped.
    pub seen: Mutex<Vec<(String, String, String)>>,
    /// Inputs passed to `authorize_url`; tests read these to assert
    /// the start handler bound the right state + challenge.
    pub authorize_seen: Mutex<Vec<(String, String, String)>>,
}

impl FakeProvider {
    /// Build a fake provider with the given id and a default
    /// verified identity.
    pub fn new(id: &'static str) -> Arc<Self> {
        Arc::new(Self {
            id,
            identity: Mutex::new(Ok(ProviderIdentity {
                provider_sub: "fake-sub-1".to_string(),
                email: "user@example.com".to_string(),
                email_verified: true,
                display_name: Some("Fake User".to_string()),
            })),
            seen: Mutex::new(Vec::new()),
            authorize_seen: Mutex::new(Vec::new()),
        })
    }

    /// Swap the identity returned on the next `fetch_identity` call.
    pub fn set_identity(&self, identity: ProviderIdentity) {
        *self.identity.lock().unwrap() = Ok(identity);
    }

    /// Force `fetch_identity` to fail with the given error.
    pub fn set_error(&self, error: ProviderError) {
        *self.identity.lock().unwrap() = Err(error);
    }
}

#[async_trait]
impl OAuthProvider for FakeProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    fn authorize_url(&self, state: &str, pkce_challenge: &str, redirect_uri: &str) -> String {
        self.authorize_seen.lock().unwrap().push((
            state.to_string(),
            pkce_challenge.to_string(),
            redirect_uri.to_string(),
        ));
        format!(
            "https://fake.example.com/{id}/authorize?state={state}&challenge={pkce_challenge}&redirect_uri={redirect_uri}",
            id = self.id,
        )
    }

    async fn fetch_identity(
        &self,
        code: &str,
        pkce_verifier: &str,
        redirect_uri: &str,
    ) -> Result<ProviderIdentity, ProviderError> {
        self.seen.lock().unwrap().push((
            code.to_string(),
            pkce_verifier.to_string(),
            redirect_uri.to_string(),
        ));
        match &*self.identity.lock().unwrap() {
            Ok(i) => Ok(i.clone()),
            Err(e) => Err(clone_error(e)),
        }
    }
}

// `ProviderError` is not `Clone`; this is the manual hand-roll for
// the small set of variants we actually use in tests.
fn clone_error(e: &ProviderError) -> ProviderError {
    match e {
        ProviderError::Transport(s) => ProviderError::Transport(s.clone()),
        ProviderError::Provider(s) => ProviderError::Provider(s.clone()),
        ProviderError::UnverifiedEmail => ProviderError::UnverifiedEmail,
        ProviderError::StateMismatch => ProviderError::StateMismatch,
    }
}

#[cfg(feature = "sqlite")]
pub use sqlite::*;

#[cfg(feature = "sqlite")]
mod sqlite {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use starter_auth_users::store::{SqliteSessionStore, SqliteUserStore};
    use starter_auth_users::Role;
    use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

    use crate::identity_store::SqliteIdentityStore;
    use crate::provider::OAuthProvider;
    use crate::routes::OAuthRoutesState;
    use crate::state_store::MemoryStateStore;

    use super::FakeProvider;

    static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
        sqlx::migrate!("../starter-auth-users/migrations/starter_auth_users");
    static AUTH_OAUTH_SQLITE_MIGRATOR: sqlx::migrate::Migrator =
        sqlx::migrate!("./migrations/starter_auth_oauth_sqlite");

    /// One-call wiring used by the integration tests. Returns a
    /// fully-populated [`OAuthRoutesState`] backed by:
    ///
    /// - sqlite-in-memory `UserStore`, `SessionStore`, and
    ///   `IdentityStore` (all on the same pool, FKs intact),
    /// - [`MemoryStateStore`] for the in-flight flow state,
    /// - the supplied set of [`FakeProvider`]s.
    pub struct MemoryEverything {
        /// The shared pool — tests can poke at it directly for
        /// fixture setup or invariants.
        pub pool: Pool,
        /// The wired routes state ready to hand to
        /// [`crate::oauth_router`].
        pub state: OAuthRoutesState,
    }

    impl MemoryEverything {
        /// Build the factory with the given providers and default
        /// settings (`signup_enabled = true`, default role `Reader`,
        /// `base_url = "https://app.example.com"`,
        /// `default_return_to = "/"`).
        pub async fn new(providers: Vec<Arc<FakeProvider>>) -> Self {
            let pool = ephemeral().await;
            migrate(&pool)
                .with_source(MigrationSource {
                    name: "starter_auth_users",
                    migrator: &AUTH_USERS_MIGRATOR,
                })
                .with_source(MigrationSource {
                    name: "starter_auth_oauth",
                    migrator: &AUTH_OAUTH_SQLITE_MIGRATOR,
                })
                .run()
                .await
                .expect("migrations apply");

            let user_store = Arc::new(SqliteUserStore::new(pool.clone()));
            let session_store = Arc::new(SqliteSessionStore::new(pool.clone()));
            let identity_store = Arc::new(SqliteIdentityStore::new(pool.clone()));
            let state_store = Arc::new(MemoryStateStore::new());

            let mut map: BTreeMap<String, Arc<dyn OAuthProvider>> = BTreeMap::new();
            for p in providers {
                map.insert(p.id().to_string(), p);
            }

            let state = OAuthRoutesState {
                providers: map,
                state_store,
                identity_store,
                user_store,
                session_store,
                base_url: "https://app.example.com".to_string(),
                signup_enabled: true,
                signup_default_role: Role::Reader,
                default_return_to: "/".to_string(),
            };

            Self { pool, state }
        }
    }
}
