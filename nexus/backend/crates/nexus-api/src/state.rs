//! Shared application state handed to every handler.
//!
//! Holds the dev datasource pool, query guards, the live runner, and the stream
//! token signer; identity, the metadata store, and per-datasource lookup join it
//! as their milestones land. `AppState` is cloneable (all fields are cheap
//! handles) so axum can share it across requests.

use std::time::Duration;

use nexus_engine::LiveRunner;
use nexus_store::QueryGuards;
use sqlx::PgPool;

use crate::middleware::StreamTokenSigner;

/// Cloneable handle bundle for the control plane.
#[derive(Clone)]
pub struct AppState {
    /// Pool against the datasource Postgres that `POST /query` runs against.
    /// Becomes a per-datasource lookup once datasource CRUD lands.
    pub datasource: PgPool,
    /// Server-enforced query bounds (read-only is applied per transaction).
    pub guards: QueryGuards,
    /// Drives unbounded live streams into the SSE broadcast.
    pub live: LiveRunner,
    /// Signs/verifies the short-lived SSE subscription tokens.
    pub stream_signer: StreamTokenSigner,
    /// Lifetime granted to a freshly-minted stream token.
    pub stream_token_ttl: Duration,
}
