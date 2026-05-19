//! Short-lived state store for the OAuth redirect flow.
//!
//! Per Hard rule R5 there is **no** DB write before the provider has
//! resolved an identity. The state store holds the only per-flow
//! state that exists between `/auth/oauth/{provider}/login` and the
//! provider's callback: the PKCE verifier, the random `state`
//! parameter, the operator-supplied `return_to`, and (for logged-in
//! linking, Hard rule R4) the user id of the currently-signed-in
//! user.
//!
//! A flow lives for [`STATE_TTL`] (10 minutes) and is **consumed on
//! read** — `take` removes it whether or not the caller succeeds.
//! That property turns the callback into the only state-changing
//! `GET` we ship (Hard rule R9) without exposing replay surface.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

mod memory;
pub use memory::MemoryStateStore;

#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStateStore;

#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "postgres")]
pub use postgres::PostgresStateStore;

/// How long a started OAuth flow remains valid. Ten minutes is the
/// upper bound a human takes to bounce through a provider's consent
/// screen on a slow connection; making it longer just widens the
/// replay window.
pub const STATE_TTL: Duration = Duration::from_secs(10 * 60);

/// One started-but-not-yet-resolved OAuth flow. Stored by `state`.
///
/// `state` is duplicated inside the struct so the in-memory and
/// future durable impls can persist a single struct without a
/// composite key.
#[derive(Debug, Clone)]
pub struct OAuthFlowState {
    /// Provider id the flow targets (`"github"`, `"google"`).
    pub provider: String,
    /// The random `state` parameter sent to the provider; the
    /// returned `state` on callback must match this byte-for-byte.
    pub state: String,
    /// PKCE verifier paired with the `code_challenge` the provider
    /// saw. The exchange step replays this; the value never leaves
    /// the callback handler.
    pub pkce_verifier: String,
    /// Where to redirect the browser after sign-in succeeds. `None`
    /// means the consumer's configured default.
    pub return_to: Option<String>,
    /// Set when the flow was started while the user was already
    /// signed in — the callback adds an identity row to **this**
    /// user instead of creating a new one (Hard rule R4).
    pub link_mode_user_id: Option<String>,
    /// Wall-clock time the flow was created; used for TTL eviction.
    pub created_at: DateTime<Utc>,
}

/// Persistence operations the callback flow needs.
///
/// `take` is the consumption point — it returns `None` for an
/// unknown, expired, or already-consumed `state`. The in-memory
/// impl also evicts every other expired entry on each `take` to
/// keep the table size bounded without a background task.
#[async_trait]
pub trait OAuthStateStore: Send + Sync {
    /// Persist a freshly-built flow keyed by its `state` field.
    /// Errors only on backend failure; an existing entry with the
    /// same `state` is overwritten — the probability of a 32-byte
    /// random collision is negligible and the alternative (error
    /// on conflict) just turns it into a hard failure for no gain.
    async fn put(&self, flow: OAuthFlowState) -> Result<(), OAuthStateError>;

    /// Look up and remove the flow keyed by `state`. Returns
    /// `Ok(None)` for unknown, expired, or already-consumed
    /// entries — the callback handler treats those uniformly as
    /// "invalid callback".
    async fn take(&self, state: &str) -> Result<Option<OAuthFlowState>, OAuthStateError>;
}

/// Failure modes any [`OAuthStateStore`] impl can surface.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OAuthStateError {
    /// Backing store failed.
    #[error("oauth state store error: {0}")]
    Backend(String),
}
