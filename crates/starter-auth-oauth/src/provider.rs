//! `OAuthProvider` — the one trait every third-party provider
//! implements, plus the [`ProviderIdentity`] every provider returns.
//!
//! Per Hard rule R6, provider-specific code lives behind this trait
//! and routes / state store / identity table stay provider-agnostic.
//! Per R7, scopes, endpoints, and required claims are compile-time
//! constants per provider — the trait does **not** carry a "scopes"
//! parameter; the impl bakes them in.

use async_trait::async_trait;

/// Normalised identity a provider returns after a successful code
/// exchange. Every provider impl converts its native userinfo
/// payload into this shape so the callback handler stays
/// provider-agnostic.
///
/// `email_verified` is load-bearing for Hard rule R3: auto-linking
/// only happens when this flag is `true`. A `false` here is the
/// only signal that lets the callback safely refuse to merge a new
/// identity onto an existing user.
#[derive(Debug, Clone)]
pub struct ProviderIdentity {
    /// Stable provider-side subject id (GitHub numeric `id`, Google
    /// `sub`). Immutable across email / display-name changes; this
    /// is the column that makes `(provider, provider_sub)` a safe
    /// primary key in `oauth_identities`.
    pub provider_sub: String,
    /// Email address the provider asserts the user owns. For
    /// GitHub this is filtered to addresses with `verified: true`
    /// in `/user/emails`; for Google it is the `email` claim paired
    /// with `email_verified: true`. Per R3 the public-profile email
    /// on GitHub is never used.
    pub email: String,
    /// Whether the provider has asserted ownership of `email`. The
    /// callback flow refuses to auto-link on `false` (Hard rule R3).
    pub email_verified: bool,
    /// Optional display name suitable for the `users.display_name`
    /// column. `None` when the provider returns nothing usable;
    /// callers fall back to the local-part of the email.
    pub display_name: Option<String>,
}

/// One third-party provider. Implementations are stateless and
/// hold only the operator-supplied client credentials + endpoint
/// constants.
///
/// The two methods below are the **only** points the callback
/// handler reaches into provider-specific code: `authorize_url`
/// generates the redirect, `fetch_identity` exchanges the code for
/// a [`ProviderIdentity`]. The access token never escapes
/// `fetch_identity` (Hard rule R2).
#[async_trait]
pub trait OAuthProvider: Send + Sync + 'static {
    /// Stable provider id (`"github"`, `"google"`). This is the
    /// value the path segment `{provider}` matches and the value
    /// stored in `oauth_identities.provider`. Must be ASCII
    /// lowercase, no spaces.
    fn id(&self) -> &'static str;

    /// Build the provider's authorization-redirect URL. `state` is
    /// the CSRF token bound to the user's browser via the state
    /// store; `pkce_challenge` is the S256-encoded challenge whose
    /// verifier is stored alongside it; `redirect_uri` is the
    /// callback URL the provider will redirect to (derived from
    /// the operator's `OAUTH_BASE_URL`).
    fn authorize_url(&self, state: &str, pkce_challenge: &str, redirect_uri: &str) -> String;

    /// Exchange `code` for a normalised [`ProviderIdentity`].
    ///
    /// Implementations make exactly the network calls they need —
    /// token exchange + userinfo (+ `/user/emails` for GitHub) —
    /// and **drop the access token before returning**. Persisting
    /// or logging it is a Hard rule R2 violation that CI greps
    /// for.
    async fn fetch_identity(
        &self,
        code: &str,
        pkce_verifier: &str,
        redirect_uri: &str,
    ) -> Result<ProviderIdentity, ProviderError>;
}

/// Failure modes a provider impl can surface during code exchange
/// or userinfo fetch.
///
/// The variants intentionally do not carry the access token, body
/// payloads, or any other secret material — every error eventually
/// becomes a `tracing` event and a user-facing HTTP response, and
/// neither is allowed to leak provider credentials (R2).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProviderError {
    /// Network transport failure reaching the provider.
    #[error("oauth provider transport error: {0}")]
    Transport(String),
    /// Provider returned a non-2xx response or an unparseable
    /// payload.
    #[error("oauth provider returned an error: {0}")]
    Provider(String),
    /// The provider returned no verified email for the user. Per
    /// R3 this is a hard refusal, not a fall-through; callers
    /// translate to `HTTP 409 email_already_registered` or to an
    /// account-creation refusal depending on flow.
    #[error("oauth provider returned no verified email")]
    UnverifiedEmail,
    /// PKCE verifier or `state` did not match what the provider
    /// returned. Almost always a forged callback.
    #[error("oauth callback state did not validate")]
    StateMismatch,
}
