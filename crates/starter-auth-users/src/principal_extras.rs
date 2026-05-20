//! `PrincipalExtrasLookup` — second one-way seam between
//! `starter-auth-users` and adjacent identity crates
//! (`starter-auth-oauth`, future SAML, future OIDC), companion to
//! [`crate::LinkedProvidersLookup`].
//!
//! `starter-auth-users` knows how to hydrate a [`Principal`] from a
//! session + user row, but the `Principal.extra` JSON bag is
//! deliberately empty there — the user-record schema does not
//! carry OAuth attributes. Authorization rules ("Google sign-ins
//! from `@acme.com` get Writer") need those attributes on the
//! principal at every request, so we expose a tiny trait the OAuth
//! crate (or any future identity-attribute source) implements; the
//! verify path calls it and merges the result into
//! `Principal.extra` before returning.
//!
//! Consumers that do not wire an attribute source get the
//! [`NoPrincipalExtras`] default — `verify_session` then returns
//! `Principal.extra == Value::Null`, identical to today.
//!
//! See `DOCS/auth/authz/SCOPE.md` R8.

use async_trait::async_trait;
use serde_json::Value;

/// Look up identity attributes that should ride on every
/// authenticated request as `Principal.extra`.
///
/// The returned value MUST be a JSON object (or `Value::Null` for
/// "no attributes"). The verify path merges its top-level keys
/// into `Principal.extra` after constructing the principal — a
/// reserved namespace like `oauth.*` lets each provider crate own
/// its own subtree without colliding with consumer-defined
/// `extra` fields.
#[async_trait]
pub trait PrincipalExtrasLookup: Send + Sync {
    /// Attributes for `user_id`. Errors surface as 500s from the
    /// verify path; impls should not bake in a "fail open"
    /// fallback because doing so would hide a misconfigured
    /// identity-attribute source behind a silent empty
    /// `Principal.extra`.
    async fn extras_for(&self, user_id: &str) -> Result<Value, PrincipalExtrasError>;
}

/// Errors a [`PrincipalExtrasLookup`] impl can surface.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PrincipalExtrasError {
    /// Backing store failed.
    #[error("principal-extras lookup error: {0}")]
    Backend(String),
}

/// Default no-op impl used when no identity-attribute source is
/// wired. Returns `Value::Null` for every user — verify keeps the
/// pre-authz behaviour exactly.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoPrincipalExtras;

#[async_trait]
impl PrincipalExtrasLookup for NoPrincipalExtras {
    async fn extras_for(&self, _user_id: &str) -> Result<Value, PrincipalExtrasError> {
        Ok(Value::Null)
    }
}
