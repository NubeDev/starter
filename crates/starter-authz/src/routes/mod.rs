//! Admin REST routes for the DB-backed policy engine.
//!
//! Mount point: `/v1/authz/*`. All routes require the caller's
//! `Principal.role == Admin`. SCOPE.md "Decisions" calls this out
//! explicitly: the admin surface is role-gated, **not**
//! permission-gated, because we cannot let a misconfigured rule
//! lock an admin out of fixing the rule that locks them out.
//!
//! Writes additionally require CSRF: the caller must echo the
//! `starter_csrf` cookie value back in an `X-CSRF-Token` header.
//! Mirrors `starter_auth_users::routes::logout` exactly.

mod assignments;
mod check;
mod resources;
mod router;
mod rules;
mod state;

pub use router::authz_router;
pub use state::AuthzRoutesState;
