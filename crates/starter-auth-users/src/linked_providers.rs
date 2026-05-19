//! `LinkedProvidersLookup` — the one-way seam between
//! `starter-auth-users` and `starter-auth-oauth`.
//!
//! `starter-auth-users` must not depend on `starter-auth-oauth` (the
//! dependency arrow is enforced by `cargo tree` in CI). When the
//! login handler hits a user row whose `password_hash` is `NULL`, it
//! needs to tell the caller "this account exists, but you have to
//! sign in through one of these providers." That list lives in the
//! OAuth crate's `oauth_identities` table; the trait below is the
//! only surface this crate exposes for it.
//!
//! Consumers that do not wire `starter-auth-oauth` get the
//! [`NoLinkedProviders`] default — it returns an empty list and the
//! `password_not_set` error still carries `providers: []`, which is
//! the right shape even when no third-party path is configured.

use async_trait::async_trait;

/// Look up which third-party providers a user has linked.
///
/// The return value is the wire shape of `LoginResponse.providers` on
/// a `password_not_set` error — each string is a provider id such as
/// `"github"` or `"google"`. Order is the caller's concern; the
/// canonical impl in `starter-auth-oauth` returns the rows in
/// `linked_at` ascending so the user sees the longest-held identity
/// first.
#[async_trait]
pub trait LinkedProvidersLookup: Send + Sync {
    /// Provider ids linked to `user_id`. Empty list if none. Errors
    /// surface as 500s from the login handler; impls should not bake
    /// in a "fail open" fallback because doing so would hide a
    /// misconfigured OAuth crate behind a stale empty list.
    async fn linked_providers(&self, user_id: &str) -> Result<Vec<String>, LinkedProvidersError>;
}

/// Errors a [`LinkedProvidersLookup`] impl can surface.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LinkedProvidersError {
    /// Backing store failed.
    #[error("linked-providers lookup error: {0}")]
    Backend(String),
}

/// Default no-op impl used when `starter-auth-oauth` is not wired.
///
/// Returns an empty list for every user. Login still surfaces the
/// `password_not_set` shape on a `NULL` hash so the SPA sees the same
/// JSON envelope regardless of whether OAuth is enabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoLinkedProviders;

#[async_trait]
impl LinkedProvidersLookup for NoLinkedProviders {
    async fn linked_providers(&self, _user_id: &str) -> Result<Vec<String>, LinkedProvidersError> {
        Ok(Vec::new())
    }
}
