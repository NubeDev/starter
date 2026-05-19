//! HTTP routes for the theme editor. Mounted by the consumer via
//! [`theme_router`].
//!
//! Authorisation contract:
//!
//! - `GET /api/v1/ui/theme` requires an authenticated [`Principal`]
//!   (any role).
//! - `GET /api/v1/ui/theme/{logo,favicon}` is **public** — these
//!   serve the bytes a browser loads via `<img src>` / favicon link,
//!   which can't ride a cookie or bearer header reliably. The bytes
//!   are non-sensitive by nature (they're the org's public logo).
//! - All mutating routes (`PUT /api/v1/ui/theme`,
//!   `POST/DELETE …/logo`, `POST/DELETE …/favicon`) require
//!   [`Role::Admin`]. The check reads the
//!   [`starter_spi::auth::Principal`] request extension produced by
//!   `starter_server::auth::with_principal` — wrap the router with
//!   `with_principal` once before mounting.
//!
//! [`Principal`]: starter_spi::auth::Principal
//! [`Role::Admin`]: starter_spi::auth::Role::Admin

mod asset_get;
mod asset_mutate;
mod errors;
mod guards;
mod router;
mod state;
mod theme_get;
mod theme_put;

pub use router::theme_router;
pub use state::ThemeState;

// The `#[utoipa::path]` macro emits a `__path_<fn>` struct in the
// same module as the handler; the `OpenApi` derive in
// `crate::openapi` resolves those types via `crate::routes::<name>`,
// so each handler-bearing module is re-exported `pub(crate)`.
pub(crate) use asset_get::{__path_get_favicon, __path_get_logo, get_favicon, get_logo};
pub(crate) use asset_mutate::{
    __path_delete_favicon, __path_delete_logo, __path_post_favicon, __path_post_logo,
    delete_favicon, delete_logo, post_favicon, post_logo,
};
pub(crate) use theme_get::{__path_get_theme, get_theme};
pub(crate) use theme_put::{__path_put_theme, put_theme};
