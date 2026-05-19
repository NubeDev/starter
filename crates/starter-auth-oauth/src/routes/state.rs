//! Shared state for the `/auth/oauth/*` handlers.
//!
//! Handlers close over an `Arc<OAuthRoutesState>` rather than
//! threading state through axum's `State` extractor — same pattern
//! `starter_auth_users::routes::auth_router` uses, kept identical so
//! a consumer can mount both routers under any `AppState` without
//! state-type gymnastics.

use std::collections::BTreeMap;
use std::sync::Arc;

use starter_auth_users::store::{SessionStore, UserStore};
use starter_auth_users::Role;

use crate::identity_store::IdentityStore;
use crate::provider::OAuthProvider;
use crate::state_store::OAuthStateStore;

/// All the bits a `/auth/oauth/*` route needs at request time.
///
/// Build once at startup. `Arc` internally so cloning is cheap;
/// callers wrap in `Arc<OAuthRoutesState>` for the router.
pub struct OAuthRoutesState {
    /// Enabled providers, keyed by [`OAuthProvider::id`]. The path
    /// segment `{provider}` looks up here; a typo is a 404 because
    /// the segment did not match a known key.
    pub providers: BTreeMap<String, Arc<dyn OAuthProvider>>,
    /// Short-lived per-flow state. Default impl is
    /// [`crate::MemoryStateStore`].
    pub state_store: Arc<dyn OAuthStateStore>,
    /// Persistent `oauth_identities` table.
    pub identity_store: Arc<dyn IdentityStore>,
    /// The users-crate user table.
    pub user_store: Arc<dyn UserStore>,
    /// The users-crate session table; minting a `sas_*` cookie on
    /// the callback ends in here (Hard rule R1).
    pub session_store: Arc<dyn SessionStore>,
    /// External-facing base URL — `redirect_uri` for each provider
    /// is built as `{base_url}/auth/oauth/{provider}/callback`.
    pub base_url: String,
    /// `false` refuses first-time sign-ins with `HTTP 403`. Phase 4
    /// honours this end-to-end (stage 9 smoke test); the wiring lives
    /// here from Phase 1 so changing the env flag is just a config
    /// bump.
    pub signup_enabled: bool,
    /// Role assigned to newly-created OAuth users when no domain
    /// map matches (the map itself lands in Phase 4).
    pub signup_default_role: Role,
    /// Where the browser lands after a successful callback when the
    /// flow's `return_to` is `None`. Usually `/`.
    pub default_return_to: String,
}
