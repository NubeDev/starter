//! # starter-auth
//!
//! Default `Authenticator` impl. Two credential paths, one
//! [`starter_spi::auth::Principal`]:
//!
//! - Browser → cookie sessions (`/auth/login`, `/auth/logout`,
//!   `/auth/me`). Passwords hashed with argon2id.
//! - Machine → API tokens (`Authorization: Bearer …`). argon2-hashed,
//!   revocable, not JWT.
//!
//! Owns its own tables (`starter_auth_users`, `starter_auth_sessions`,
//! `starter_auth_tokens`) shipped as migrations under source
//! `starter_auth`.
//!
//! - [`authenticator`] — the `Authenticator` impl bridging both paths.
//! - [`role`], [`scope`] — built-in role + scope sets.
//! - [`routes`] — `/auth/*` axum routes the consumer mounts.
//! - [`password`] — argon2id hash + verify.
//! - [`session`] — cookie session store + lifecycle.
//! - [`token`] — API token issue + verify.
//! - [`guard`] — `require_role` / `require_scope` middleware
//!   factories.
//! - [`admin`] — programmatic admin user creation (CLI uses this).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod admin;
pub mod authenticator;
pub mod guard;
pub mod password;
pub mod role;
pub mod routes;
pub mod scope;
pub mod session;
pub mod token;

pub use authenticator::AuthAuthenticator;
pub use role::Role;
pub use scope::Scope;
