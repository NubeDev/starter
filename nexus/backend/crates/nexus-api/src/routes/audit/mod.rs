//! Audit read API (WS-12 §3.4) — the changelog projected as an audit log.
//!
//! Audit and undo share one ledger; this module is the read side. Both routes
//! are **admin-gated** (audit is privileged: a tenant admin sees their own
//! tenant's log, a super-admin sees across tenants) and **tenant-isolated** via
//! the per-request [`nexus_store::changelog::NexusChangeLog`], whose every query
//! runs inside a tenant transaction so RLS filters rows to the caller's tenant by
//! construction. Read-only verbs never reach here — there is nothing to record.

pub mod forget;
mod gate;
pub mod list;
pub mod resource;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

/// `/api/v1/audit` (filtered list), `/api/v1/audit/resources/{kind}/{id}`
/// (single-resource history timeline), and `/api/v1/audit/forget` (GDPR
/// right-to-erasure for a user subject). All three are admin-gated.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/audit", get(list::list_audit))
        .route(
            "/api/v1/audit/resources/{kind}/{id}",
            get(resource::resource_history),
        )
        .route("/api/v1/audit/forget", post(forget::forget_subject))
}
