//! Shared application state handed to every handler.
//!
//! Holds the metadata pool (where the control plane's own tenant-scoped tables
//! live), the dev datasource pool, the secret envelope, query guards, the live
//! runner, and the stream token signer. Per-datasource pool lookup replaces the
//! single `datasource` handle as more connectors land. `AppState` is cloneable
//! (all fields are cheap handles) so axum can share it across requests.

use std::sync::Arc;
use std::time::Duration;

use nexus_engine::{FlowManager, LiveRunner};
use nexus_store::datasource::Envelope;
use nexus_store::QueryGuards;
use sqlx::PgPool;
use starter_spi::authz::PolicyEngine;

use crate::datasource_pools::DatasourcePools;
use crate::kinds::Registry as KindRegistry;
use crate::middleware::StreamTokenSigner;

/// Cloneable handle bundle for the control plane.
#[derive(Clone)]
pub struct AppState {
    /// The control plane's own Postgres — datasources, dashboards, panels — under
    /// the non-BYPASSRLS runtime role. All tenant-scoped tables live here.
    pub metadata: PgPool,
    /// Pool against the datasource Postgres that `POST /query` runs against.
    /// The dev single-datasource shortcut; `POST /datasources/:id/query` uses the
    /// per-datasource cache below instead.
    pub datasource: PgPool,
    /// Per-datasource connection pools, built on first query and reused after.
    /// Keyed on the immutable datasource id within its tenant (R5).
    pub datasource_pools: DatasourcePools,
    /// Envelope used to seal/open datasource connection secrets.
    pub envelope: Envelope,
    /// Server-enforced query bounds (read-only is applied per transaction).
    pub guards: QueryGuards,
    /// Drives unbounded live streams into the SSE broadcast.
    pub live: LiveRunner,
    /// Runs saved ingestion flows as long-lived streams, keyed by flow id.
    pub flows: FlowManager,
    /// Drives AI agent sessions and feeds their SSE subscribers, keyed by
    /// session id. Wraps the nexus-ai facade.
    pub sessions: crate::agents::SessionRunner,
    /// Signs/verifies the short-lived SSE subscription tokens.
    pub stream_signer: StreamTokenSigner,
    /// Lifetime granted to a freshly-minted stream token.
    pub stream_token_ttl: Duration,
    /// Grant-check engine. In the running server this is the very
    /// `DbPolicyEngine` the `/v1/authz/*` router writes to, so a grant created
    /// through the API is visible to the next handler check; tests can swap in
    /// `AllowAll`/`DenyAll` to assert a route is gated.
    pub engine: Arc<dyn PolicyEngine>,
    /// Registered declarative query-kinds (WS-10), loaded from the built-in pack
    /// at boot. A kind-mode query resolves its name here, validates params, and
    /// binds the kind's SQL through the shared binder. Shared read-only across
    /// requests; an empty registry means kind-mode requests 404.
    pub kinds: Arc<KindRegistry>,
}
