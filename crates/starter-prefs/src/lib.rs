//! `starter-prefs` — user / org preferences (locale, units, timezone,
//! currency) with three-layer resolution.
//!
//! Owns: SCOPE.md "Preferences model", "API surface", and the
//! Phase 1 entries in "Rollout (proposed phases)" in
//! `DOCS/user/scope/SCOPE.md`.
//!
//! Stage 3 of the user-prefs / i18n job lands this as an empty
//! scaffold: the module tree, feature gates, and dep graph. Later
//! stages fill in resolver, store, routes, and middleware.

/// Three-layer resolver: user → org → default, with `"auto"`
/// derivation per **R3 — Three-layer resolution** in
/// `DOCS/user/scope/SCOPE.md`.
pub mod resolver;

/// `PrefsStore` trait + sqlite impl. Postgres is deferred per the
/// SCOPE.md "Decisions" Phase 1 lock (sqlite-only for this job;
/// Postgres in a follow-up). Owned by SCOPE.md
/// "Preferences model" + "Crate layout".
pub mod store;

/// REST surface: `GET/PATCH /v1/me/preferences`,
/// `GET/PATCH /v1/orgs/{id}/preferences`, `GET /v1/units`. Owned by
/// SCOPE.md "API surface". Gated behind the `routes` feature so
/// headless / non-HTTP consumers don't pull axum in.
#[cfg(feature = "routes")]
pub mod routes;

/// Tower middleware that resolves prefs once per request and threads
/// the result into request extensions for downstream handlers (and
/// for the `Accept-Units` middleware landing in Phase 2). Owned by
/// SCOPE.md "Middleware". Gated behind `routes` for the same reason.
#[cfg(feature = "routes")]
pub mod middleware;
