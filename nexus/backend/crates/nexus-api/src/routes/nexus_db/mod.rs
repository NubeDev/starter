//! Admin read-only inspector for the control-plane (metadata) database.
//!
//! `POST /api/v1/nexus-db/query` runs raw SQL against the nexus metadata pool —
//! the same database `make db` serves, holding users/datasources/flows/etc. —
//! rather than a registered datasource. It is the platform's own DB, so access
//! is locked down on three independent axes:
//!
//! - **Admin only**: a non-admin principal is refused 403 before any SQL runs.
//! - **Tenant-scoped**: the transaction binds `app.tenant_id`, so RLS filters
//!   every row to the caller's tenant — an admin still sees only their tenant's
//!   control-plane rows, never another tenant's.
//! - **Read-only + capped**: the transaction is `READ ONLY` (writes/DDL are
//!   rejected by Postgres itself) under the same statement-timeout and row/byte
//!   caps as the datasource query path.
//!
//! This is deliberately *not* the datasource Explore path: that runs against
//! `state.datasource`; this runs against `state.metadata`.

pub mod query;

use axum::Router;

use crate::state::AppState;

/// The `/api/v1/nexus-db` surface.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/nexus-db/query", axum::routing::post(query::query_nexus_db))
}
