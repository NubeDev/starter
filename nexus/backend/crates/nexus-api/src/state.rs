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

use crate::cache::QueryCache;
use crate::changelog::ChangelogHandles;
use crate::datasource_kinds::Registry as DatasourceKindRegistry;
use crate::datasource_pools::DatasourcePools;
use crate::kinds::Registry as KindRegistry;
use crate::middleware::StreamTokenSigner;
use crate::prefs::NexusPrefs;
use crate::quota::TenantQuotas;
use crate::ratelimit::TenantRateLimiter;

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
    /// Extension-contributed query-kinds (WS-14) — the dispatcher's *third*
    /// source, beside the file pack (`kinds`) and the per-tenant overlay
    /// (`nexus_query_kinds`). Built at boot from the `nexus_extension_query_kinds`
    /// provenance table: an installed extension's `warehouse_templates[]` are
    /// materialised here so the dispatcher resolves them through the identical
    /// validate/bind path as file kinds, with no per-request DB hit. Global
    /// (extensions install once per deployment); an empty registry means no
    /// extension contributes a kind. Replaced wholesale on the next boot after an
    /// install/uninstall (the registry is sealed-by-restart, like the file pack).
    pub extension_kinds: Arc<KindRegistry>,
    /// The sealed extension registry (WS-14/WS-17) — every validated bundle's
    /// `ExtensionRecord` (manifest + bundle dir). Host methods that must consult
    /// the **calling** extension's manifest at request time read it from here:
    /// `warehouse.write` builds its per-call own-table allowlist from the
    /// caller's `contributes.warehouse_tables[]`, and `datasource.*` reads the
    /// caller's declared datasource grant. Built once at boot and shared
    /// read-only (sealed-by-restart, like the kind registries).
    pub extensions: Arc<starter_ext_host::ExtensionRegistry>,
    /// Registered declarative datasource-kinds (WS-08b), loaded from the built-in
    /// pack at boot. A connector type (`postgres`, `mqtt`) declared by manifest:
    /// its config schema validates a config before save, its `secret_fields`
    /// drive the seal boundary, its `test` descriptor selects the probe path. The
    /// catalogue route reads this so the UI renders per-kind config forms. Shared
    /// read-only across requests; an empty registry means no connector is declared.
    pub datasource_kinds: Arc<DatasourceKindRegistry>,
    /// User/org preference handles (WS-11): the Postgres store + system defaults
    /// backing both `/me/preferences` and the `Accept-Units` units-conversion
    /// middleware. `workspace_id` is the caller's tenant; storage is route-pinned
    /// to it for isolation.
    pub prefs: NexusPrefs,
    /// Audit/undo substrate (WS-12): the boot-built reversible registry and redo
    /// cursor. Per-request tenant-pinned logs/recorders are built from these plus
    /// the metadata pool; the audit and undo routes go through them.
    pub changelog: ChangelogHandles,
    /// Query result cache (WS-09 P1): an in-process TTL cache keyed by the full
    /// C3 tuple, with single-flight coalescing so a dashboard's refresh burst of
    /// identical panel queries makes one database round-trip per key per tick.
    pub query_cache: QueryCache,
    /// Per-tenant query concurrency caps (WS-09 P1): a query is admitted through
    /// the calling tenant's semaphore before it runs, so one tenant's fan-out
    /// cannot exhaust the pool and starve others.
    pub quotas: TenantQuotas,
    /// Per-tenant request rate limiter (WS-09 P1): the token-bucket the
    /// rate-limit middleware applies. Held on state so `serve::assemble` mounts
    /// the same instance tests can drive.
    pub rate_limiter: TenantRateLimiter,
    /// Runtime liveness canary (WS-16): an atomic the canary tick task bumps once
    /// per second. The `/livez` and `/readyz` routes read it to distinguish a
    /// wedged tokio runtime from a healthy-but-slow one. Cheap clone of an
    /// `Arc<AtomicU64>`.
    pub canary: crate::boot::runtime_canary::Canary,
}
