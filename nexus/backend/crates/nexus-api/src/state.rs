//! Shared application state handed to every handler.
//!
//! Holds the metadata pool (where the control plane's own tenant-scoped tables
//! live), the dev datasource pool, the secret envelope, query guards, the live
//! runner, and the stream token signer. Per-datasource pool lookup replaces the
//! single `datasource` handle as more connectors land. `AppState` is cloneable
//! (all fields are cheap handles) so axum can share it across requests.

use std::time::Duration;

use nexus_engine::LiveRunner;
use nexus_store::datasource::Envelope;
use nexus_store::QueryGuards;
use sqlx::PgPool;

use crate::middleware::StreamTokenSigner;

/// Cloneable handle bundle for the control plane.
#[derive(Clone)]
pub struct AppState {
    /// The control plane's own Postgres — datasources, dashboards, panels — under
    /// the non-BYPASSRLS runtime role. All tenant-scoped tables live here.
    pub metadata: PgPool,
    /// Pool against the datasource Postgres that `POST /query` runs against.
    /// Becomes a per-datasource lookup once multiple datasources are wired.
    pub datasource: PgPool,
    /// Envelope used to seal/open datasource connection secrets.
    pub envelope: Envelope,
    /// Server-enforced query bounds (read-only is applied per transaction).
    pub guards: QueryGuards,
    /// Drives unbounded live streams into the SSE broadcast.
    pub live: LiveRunner,
    /// Signs/verifies the short-lived SSE subscription tokens.
    pub stream_signer: StreamTokenSigner,
    /// Lifetime granted to a freshly-minted stream token.
    pub stream_token_ttl: Duration,
}
