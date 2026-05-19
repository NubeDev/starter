//! # starter-auth-users
//!
//! Multi-user `Authenticator` impl. Two credential paths, one
//! [`starter_spi::auth::Principal`]. SCOPE 228–232: mutually
//! exclusive with `starter-auth-token` — a consumer wires one or the
//! other.
//!
//! - Browser → cookie sessions (`/auth/login`, `/auth/logout`,
//!   `/auth/me`). Passwords hashed with argon2id, sessions backed by
//!   `starter_auth_users_sessions`, CSRF via double-submit cookie.
//! - Machine → API tokens (`Authorization: Bearer …`). Token format
//!   `sak_<public_id>.<secret>`; the secret half is argon2id-hashed at
//!   rest, the public id is the table key (O(1) lookup).
//!
//! Owns its own tables (`starter_auth_users_users`,
//! `starter_auth_users_sessions`, `starter_auth_users_tokens`)
//! shipped as migrations under source `starter_auth_users`.
//!
//! - [`authenticator`] — the `Authenticator` impl bridging both paths.
//! - [`role`], [`scope`] — built-in role + scope sets (re-export the
//!   spi types so existing import paths keep working).
//! - [`routes`] — `/auth/*` axum routes the consumer mounts.
//! - [`password`] — argon2id hash + verify.
//! - [`session`] — cookie session issue / verify / revoke.
//! - [`token`] — API token issue / verify / revoke.
//! - [`store`] — `UserStore` / `SessionStore` / `TokenStore` traits
//!   with `feature = "sqlite"` impls.
//! - [`admin`] — programmatic admin user creation (CLI uses this).
//!
//! `require_role` / `require_scope` middleware now live in
//! `starter-server` (SCOPE 458–460) — they're parameterised over the
//! `Authenticator` trait so consumers wiring `starter-auth-token`
//! get the same guards. See
//! `starter_server::auth::{with_principal, with_role, with_scope}`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod admin;
pub mod authenticator;
pub mod password;
pub mod role;
pub mod routes;
pub mod scope;
pub mod session;
pub mod store;
pub mod token;

pub use authenticator::AuthAuthenticator;
pub use role::Role;
pub use scope::Scope;
