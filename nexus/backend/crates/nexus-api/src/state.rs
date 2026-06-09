//! Shared application state handed to every handler.
//!
//! M0 holds just the dev datasource pool and the query guards; identity, the
//! metadata store, the engine handles, and the stream registry join it as their
//! milestones land. `AppState` is cloneable (all fields are cheap handles) so
//! axum can share it across requests.

use nexus_store::QueryGuards;
use sqlx::PgPool;

/// Cloneable handle bundle for the control plane.
#[derive(Clone)]
pub struct AppState {
    /// Pool against the datasource Postgres that `POST /query` runs against.
    /// Becomes a per-datasource lookup once datasource CRUD lands.
    pub datasource: PgPool,
    /// Server-enforced query bounds (read-only is applied per transaction).
    pub guards: QueryGuards,
}
