//! # starter-ui-theme
//!
//! Backend half of the theme editor documented in
//! [`DOCS/frontend/theme/README.md`](../../DOCS/frontend/theme/README.md).
//! Ships:
//!
//! - [`store`] — [`starter_spi::ui::theme::ThemeStore`] impls for
//!   sqlite (`feature = "sqlite"`) and Postgres (`feature = "postgres"`).
//!   Both back assets as BLOBs in the same single-row table the
//!   styles live in — no filesystem coupling, round-trip tests stay
//!   trivial, S3 / MinIO can land as a parallel impl when a consumer
//!   asks (per TODO Phase 9b).
//! - [`routes`] — the six HTTP handlers the frontend's
//!   `httpThemeTransport` calls, plus a `theme_router` builder
//!   (`feature = "routes"`, default-on).
//! - [`openapi`] — utoipa `OpenApi` derive for the routes the
//!   consumer merges into the doc they serve.
//! - [`migrations`](../../crates/starter-ui-theme/migrations) — one
//!   migration per backend, source-named `ui_theme`. Apply via
//!   `starter_store_sqlite::migrate` / the Postgres equivalent.
//!
//! Asset URLs returned in [`starter_spi::ui::theme::ThemeDocument`]
//! point at this crate's own `GET /api/v1/ui/theme/{logo,favicon}`
//! handlers — no consumer-side static-files wiring required.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod store;

#[cfg(feature = "routes")]
pub mod routes;

#[cfg(feature = "routes")]
pub mod openapi;

/// Asset size limits enforced by the route handlers. Mirrors what
/// `DOCS/frontend/theme/README.md` promises ("Asset limits the
/// frontend enforces").
pub mod limits {
    /// Maximum logo payload (PNG / SVG).
    pub const LOGO_MAX_BYTES: usize = 256 * 1024;
    /// Maximum favicon payload (PNG / ICO).
    pub const FAVICON_MAX_BYTES: usize = 64 * 1024;
}

/// Accepted MIME types per asset.
pub mod accepted_mime {
    /// MIME types accepted for the logo upload endpoint.
    pub const LOGO: &[&str] = &["image/png", "image/svg+xml"];
    /// MIME types accepted for the favicon upload endpoint.
    pub const FAVICON: &[&str] = &[
        "image/png",
        "image/x-icon",
        "image/vnd.microsoft.icon",
    ];
}

/// URL paths the GET asset endpoints are served at. Stored back into
/// [`starter_spi::ui::theme::ThemeDocument::logo_url`] / `favicon_url`
/// so the frontend can render the bytes without knowing the route
/// shape.
pub mod asset_urls {
    /// Path the logo handler is mounted at.
    pub const LOGO: &str = "/api/v1/ui/theme/logo";
    /// Path the favicon handler is mounted at.
    pub const FAVICON: &str = "/api/v1/ui/theme/favicon";
}
