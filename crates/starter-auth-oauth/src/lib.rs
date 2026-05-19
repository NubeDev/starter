//! # starter-auth-oauth
//!
//! Third-party sign-up / sign-in via OAuth 2.0 / OIDC providers
//! (GitHub + Google in v0.1, extensible via the [`OAuthProvider`]
//! trait). Sits **next to** `starter-auth-users` and **reuses** its
//! `SessionStore`: once the callback resolves a third-party identity
//! into a `UserRecord`, every downstream auth check (Principal, Role,
//! Scope, CSRF) is unchanged.
//!
//! Module layout per the source SCOPE §"Repo layout":
//!
//! - [`provider`] — the trait every provider implements + the
//!   normalised [`ProviderIdentity`] every provider returns.
//! - [`state_store`] — the short-lived [`OAuthFlowState`] store; the
//!   in-memory default lives here, sqlite + postgres impls land in
//!   Phase 4.
//! - [`identity_store`] — read/write the new `oauth_identities`
//!   table; sqlite impl is feature-gated.
//! - [`linked_providers`] — the
//!   [`starter_auth_users::LinkedProvidersLookup`] impl that closes
//!   the one-way seam between this crate and the users crate.
//! - [`config`] — env-var + `starter-secrets-*` configuration.
//!
//! Higher-level pieces (`providers/github`, `providers/google`,
//! `routes`, `session_bridge`) land in later stages; this file only
//! re-exports what is wired today.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod identity_store;
pub mod linked_providers;
pub mod provider;
pub mod state_store;

pub use config::{OAuthConfig, OAuthConfigError, ProviderCredentials, StateStoreKind};
pub use identity_store::{IdentityStore, IdentityStoreError, OAuthIdentity};
#[cfg(feature = "sqlite")]
pub use identity_store::SqliteIdentityStore;
pub use linked_providers::OAuthLinkedProviders;
pub use provider::{OAuthProvider, ProviderError, ProviderIdentity};
pub use state_store::{
    MemoryStateStore, OAuthFlowState, OAuthStateError, OAuthStateStore, STATE_TTL,
};
