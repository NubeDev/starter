//! `IdentityStore` — read/write the `oauth_identities` table.
//!
//! Owns one table: `starter_auth_oauth_identities`, keyed by the
//! composite primary key `(provider, provider_sub)`. The shape is
//! locked in by migration `0001_oauth_identities.sql`; this file
//! is the trait + impls.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteIdentityStore;

/// One row in `starter_auth_oauth_identities`.
///
/// `email` and `display_name` are cached from the most recent
/// successful sign-in; they are not authoritative — the provider's
/// userinfo on the next sign-in is. They exist so the
/// `GET /auth/oauth/identities` endpoint can render a useful list
/// without re-hitting the provider on every page load.
#[derive(Debug, Clone)]
pub struct OAuthIdentity {
    /// Provider id (`"github"`, `"google"`).
    pub provider: String,
    /// Stable provider-side subject id. Half of the composite key.
    pub provider_sub: String,
    /// Local user this identity is linked to. FK to
    /// `starter_auth_users_users(id)` with `ON DELETE CASCADE`.
    pub user_id: String,
    /// Email the provider returned at last sign-in.
    pub email: Option<String>,
    /// Display name the provider returned at last sign-in.
    pub display_name: Option<String>,
    /// Wall-clock time the row was created (the link / signup
    /// timestamp). Used to surface the longest-held identity
    /// first in `GET /auth/oauth/identities`.
    pub linked_at: DateTime<Utc>,
}

/// Errors specific to identity persistence.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IdentityStoreError {
    /// `(provider, provider_sub)` is already linked.
    #[error("oauth identity already linked")]
    Conflict,
    /// Backing store failed.
    #[error("oauth identity store error: {0}")]
    Backend(String),
}

/// Persistence operations the OAuth callback + admin endpoints
/// need. Kept narrow: this trait does **not** expose update — when
/// a provider returns a changed email we delete + insert in the
/// link flow so the email-change tracing event has a clean place
/// to fire (SCOPE Decisions §"Email-change-as-security-event").
#[async_trait]
pub trait IdentityStore: Send + Sync {
    /// Find by composite key. `None` on miss.
    async fn find(
        &self,
        provider: &str,
        provider_sub: &str,
    ) -> Result<Option<OAuthIdentity>, IdentityStoreError>;

    /// Insert a new identity row. Returns `Conflict` when the
    /// composite key already exists.
    async fn insert(&self, identity: &OAuthIdentity) -> Result<(), IdentityStoreError>;

    /// Delete by composite key. Idempotent — deleting a missing
    /// row is not an error.
    async fn delete(
        &self,
        provider: &str,
        provider_sub: &str,
    ) -> Result<(), IdentityStoreError>;

    /// List every identity linked to a user, oldest first. Used by
    /// both `GET /auth/oauth/identities` and by the
    /// `LinkedProvidersLookup` impl that powers the
    /// `password_not_set` login error.
    async fn list_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<OAuthIdentity>, IdentityStoreError>;
}
