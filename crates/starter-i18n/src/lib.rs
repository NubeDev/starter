//! `starter-i18n` — internationalisation: catalog loader, language
//! negotiation, translate bundle, and (behind feature gates) the REST
//! surface + tower middleware for the platform.
//!
//! Owns: SCOPE.md Phase 3 — catalog format, `Accept-Language`
//! negotiation, `GET /v1/i18n/catalogs/{lang}`,
//! `GET /v1/i18n/manifest`, and the seed catalogs at
//! `catalogs/starter/`. See `DOCS/user/scope/SCOPE.md`.
//!
//! Stage 12 of the user-prefs / i18n job lands this as the crate
//! scaffold + the locale-negotiation module. Later stages fill in
//! the catalog loader, translate bundle, REST routes, and the
//! Phase 5 diagnostics rewriter.

/// BCP-47-aware locale negotiation: `Accept-Language` parsing and
/// the R5 fallback walk (requested → language family → `en`).
pub mod locale;

/// Catalog format + loader. Plain ICU JSON keyed by
/// `starter_spi::i18n::MessageKey`; loader uses `deny_unknown_fields`
/// per the SCOPE Phase 3 "Decisions" lock. Empty in stage 12; filled
/// in by the next stage.
pub mod catalog;

/// `TranslateBundle` — an in-memory snapshot of the loaded catalogs
/// keyed by `LanguageTag`. Built once at startup; cloned cheaply via
/// `Arc`. Empty in stage 12; filled in by the next stage.
pub mod bundle;

/// The `Translate` trait: the small read-only surface a handler
/// reaches for. Empty in stage 12; filled in by the next stage.
pub mod translate;

/// Platform seed catalogs at `catalogs/starter/` (en / es). The
/// loader resolves the directory either from a configurable root or
/// from this crate's embedded fallback. Empty in stage 12; filled in
/// by the next stage.
pub mod platform;

/// REST surface: `GET /v1/i18n/catalogs/{lang}`,
/// `GET /v1/i18n/manifest`. Owned by SCOPE.md Phase 3 "API surface".
/// Gated behind the `routes` feature so headless / non-HTTP consumers
/// don't pull axum in. Empty in stage 12; filled in by the next stage.
#[cfg(feature = "routes")]
pub mod routes;

/// Tower middleware: parses `Accept-Language`, resolves the caller's
/// language via [`locale::pick_language`], and inserts the choice
/// into request extensions for downstream handlers. Owned by
/// SCOPE.md Phase 3 "Middleware". Gated behind `routes` for the same
/// reason. Empty in stage 12; filled in by the next stage.
#[cfg(feature = "routes")]
pub mod middleware;
