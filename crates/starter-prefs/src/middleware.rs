//! Tower middleware that resolves prefs once per request and stashes
//! the result in request extensions.
//!
//! Owns: SCOPE.md "Middleware" — the prefs-resolution layer that the
//! Phase 2 `Accept-Units` middleware in starter-server reads from
//! when building `UnitsCtx`. Gated behind the `routes` cargo feature
//! (default off) because middleware pulls axum/tower into the dep
//! tree. Empty in stage 3; lands in stage 7 alongside the route
//! handlers.
