//! REST surface for preferences.
//!
//! Owns: SCOPE.md "API surface" — the four endpoints
//! `GET/PATCH /v1/me/preferences`, `GET/PATCH /v1/orgs/{id}/preferences`,
//! and `GET /v1/units`. Gated behind the `routes` cargo feature
//! (default off) so headless consumers stay axum-free per the
//! workspace policy posture in SCOPE.md "Rollout (proposed phases)".
//! Empty in stage 3; handlers land in stage 7.
