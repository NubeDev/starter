//! Always-on flow runtime wiring (Stage C.2 of the live-tick demo).
//!
//! Owns three pieces of cross-cutting state shared by the rest of the
//! agent process:
//!
//! 1. **`FlowSubscriptionRegistry`** — a `HashMap<FlowId,
//!    broadcast::Sender<FlowEvent>>` shared between the per-flow run
//!    pump and the `GET /api/v1/flows/{flow_id}/events` SSE route
//!    ([`crate::routes::flow_events`]). The route holds a clone of the
//!    `Arc` and calls [`FlowSubscriptionRegistry::subscribe_or_create`]
//!    on every request; the engine-side producer (wired in a later
//!    stage as it consumes `RunHandle::events_tx`) fan-outs `FlowEvent`s
//!    into the same sender so the SSE consumers see them.
//!
//! 2. **`NodeStateStore`** — the SPI seam added in Stage 0
//!    (`DOCS/flow/scope/node-state.md`). When the caller hands in a
//!    [`PgPool`] we apply the upstream flow migrations (including
//!    `0002_node_state.sql`) against it and wrap the pool in
//!    [`PgNodeStateStore`]. Otherwise we fall back to
//!    [`InMemoryNodeStateStore`] so a laptop boot without Postgres
//!    still has a working seam (at the cost of volatile state).
//!
//!    The shared PG pool comes from `main.rs` (the same pool the
//!    MCP surface uses for `flows_definitions` seed/load), so this
//!    boot path no longer opens a second connection pool just for
//!    node-state.
//!
//!    Legacy in-flight state migration: when a `~/.rubix/node_state.db`
//!    file exists at boot — left behind by older rubix-agent builds
//!    that used `SqliteNodeStateStore` — we copy any rows into the
//!    new PG `node_state` table on a first-writer-wins basis
//!    (`ON CONFLICT DO NOTHING`) and rename the file to `*.migrated`
//!    so subsequent boots short-circuit. See
//!    `rubix/docs/scope/sqlite-to-postgres.md` Block 1.
//!
//! 3. **Bundled-schedule enumeration** — generalises the
//!    weekly-report wiring from `boot/scheduler.rs`. The legacy
//!    `boot::scheduler::spawn` now delegates to
//!    [`bundled_schedule_pairs`] so both the new always-on mounter and
//!    the existing tick loop see the same `(flow_id, cron_expr)` list.
//!
//! The actual per-flow event pump (subscribing each `RunHandle` and
//! forwarding to the broadcast) is a follow-up stage — what lands here
//! is the registry seam, the NodeStateStore wiring, and the
//! bundled-schedule helper. The SSE route therefore emits heartbeats
//! today and starts emitting `NodeSlotValue` payloads the moment the
//! pump lands; no API change required on the wire.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{ConnectOptions, Row};
use tokio::sync::broadcast;
use std::sync::RwLock;
use tracing::{info, warn};

use starter_flow::state::in_memory::InMemoryNodeStateStore;
use starter_flow_spi::flow::{FlowEvent, FlowId};
use starter_flow_spi::state::NodeStateStore;
use starter_store_postgres::flow::node_state::PgNodeStateStore;
use starter_store_postgres::flow::FLOW_MIGRATION_SOURCE;
use starter_store_postgres::migrate;
use starter_store_postgres::pool::Pool as PgPool;

use crate::boot::config::FlowRuntimeConfig;

/// Default capacity for each per-flow `FlowEvent` broadcast channel.
/// Matches the per-run broadcast default in `FlowRunnerConfig`.
const BROADCAST_CAPACITY: usize = 256;

/// Cross-process bookkeeping shared between the engine-side event
/// pump and the SSE route. Cheap to clone; the [`RwLock`] only holds
/// the per-flow `Sender` map.
#[derive(Default)]
pub struct FlowSubscriptionRegistry {
    inner: RwLock<std::collections::HashMap<FlowId, broadcast::Sender<FlowEvent>>>,
}

impl FlowSubscriptionRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe to the broadcast for `flow_id`, lazily creating the
    /// channel on first call. The returned [`broadcast::Receiver`]
    /// observes every [`FlowEvent`] sent through the matching sender
    /// from the moment the receiver was created — i.e. SSE clients
    /// connecting mid-run only see events emitted after they
    /// subscribe (matches the per-run broadcast semantics upstream).
    pub async fn subscribe_or_create(&self, flow_id: &FlowId) -> broadcast::Receiver<FlowEvent> {
        // Fast path: already-existing sender.
        if let Some(tx) = self
            .inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(flow_id)
        {
            return tx.subscribe();
        }
        // Slow path: install a fresh sender. The race window is
        // benign — a second concurrent caller may have inserted
        // first; we re-check under the write lock and reuse.
        let mut map = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let tx = map
            .entry(flow_id.clone())
            .or_insert_with(|| broadcast::channel::<FlowEvent>(BROADCAST_CAPACITY).0)
            .clone();
        tx.subscribe()
    }

    /// Borrow (or create) the broadcast sender for `flow_id`. The
    /// engine-side event pump calls this when it wires a freshly-
    /// started run's `events_tx` into the per-flow fan-out.
    pub async fn sender(&self, flow_id: &FlowId) -> broadcast::Sender<FlowEvent> {
        if let Some(tx) = self
            .inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(flow_id)
        {
            return tx.clone();
        }
        let mut map = self.inner.write().unwrap_or_else(|e| e.into_inner());
        map.entry(flow_id.clone())
            .or_insert_with(|| broadcast::channel::<FlowEvent>(BROADCAST_CAPACITY).0)
            .clone()
    }
}

/// `FlowEventSink` implementation: feeds every event into the
/// per-flow broadcast that the SSE route subscribes to.
impl starter_flow::FlowEventSink for FlowSubscriptionRegistry {
    fn publish(&self, flow: &FlowId, event: FlowEvent) {
        // Re-use the fast path of `sender()` synchronously —
        // `tokio::sync::broadcast::Sender::send` itself is sync.
        let tx = {
            if let Some(tx) = self
                .inner
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .get(flow)
            {
                tx.clone()
            } else {
                let mut map = self.inner.write().unwrap_or_else(|e| e.into_inner());
                map.entry(flow.clone())
                    .or_insert_with(|| broadcast::channel::<FlowEvent>(BROADCAST_CAPACITY).0)
                    .clone()
            }
        };
        // `Err` only when there are no receivers — fine to drop.
        let _ = tx.send(event);
    }
}

/// Owned bundle returned by [`build`]. Threaded into [`crate::routes`]
/// and (in a follow-up stage) into the engine-side run pump.
#[derive(Clone)]
pub struct FlowRuntime {
    /// Per-flow `FlowEvent` broadcast registry.
    pub subscriptions: Arc<FlowSubscriptionRegistry>,
    /// Persistent state seam threaded into every `NodeCtx::state`.
    pub state_store: Arc<dyn NodeStateStore>,
}

/// Construct the runtime: pick the right [`NodeStateStore`] backend
/// based on the supplied PG pool and create an empty
/// [`FlowSubscriptionRegistry`].
///
/// Selection rule:
///
/// - `pg_pool = Some(_)` — apply the upstream
///   [`FLOW_MIGRATION_SOURCE`] against the pool (creates the
///   `node_state` table if missing) and wrap it in
///   [`PgNodeStateStore`]. Best-effort migrate any rows surviving
///   in the legacy `~/.rubix/node_state.db` file via
///   [`migrate_legacy_node_state_db`] before returning.
/// - Otherwise — fall back to [`InMemoryNodeStateStore`] and log so
///   the operator can see they're on the volatile path.
///
/// The `_cfg` parameter is retained for forward compatibility; the
/// runtime carries no per-instance tunables today (the previous
/// `state_db_path` knob was removed when node state moved into
/// Postgres — see `rubix/docs/scope/sqlite-to-postgres.md`).
pub async fn build(
    pg_pool: Option<PgPool>,
    _cfg: &FlowRuntimeConfig,
) -> Result<FlowRuntime> {
    let state_store: Arc<dyn NodeStateStore> = match pg_pool {
        Some(pool) => {
            migrate::migrate(&pool)
                .with_source(FLOW_MIGRATION_SOURCE)
                .run()
                .await
                .map_err(|e| anyhow::anyhow!("apply flow migrations to Postgres: {e}"))?;

            // Best-effort migrate any rows from the legacy
            // SQLite file into PG, then rename the file so this
            // step short-circuits on subsequent boots. Failures
            // are logged but never abort boot — the operator can
            // re-run `scripts/migrate-node-state-to-pg.sh`
            // explicitly. Semantics: first-writer-wins; rows
            // already in PG are never overwritten.
            if let Err(e) = migrate_legacy_node_state_db(&pool).await {
                warn!(
                    target: "rubix.boot.flow_runtime",
                    error = %e,
                    "legacy ~/.rubix/node_state.db migration failed — \
                     boot continues; re-run scripts/migrate-node-state-to-pg.sh",
                );
            }

            info!(
                target: "rubix.boot.flow_runtime",
                "NodeStateStore: Postgres (durable)",
            );
            Arc::new(PgNodeStateStore::new(pool))
        }
        None => {
            warn!(
                target: "rubix.boot.flow_runtime",
                "NodeStateStore: in-memory (volatile) — set RUBIX_DATABASE_URL for durability",
            );
            Arc::new(InMemoryNodeStateStore::new())
        }
    };
    Ok(FlowRuntime {
        subscriptions: Arc::new(FlowSubscriptionRegistry::new()),
        state_store,
    })
}

/// Path of the legacy SQLite node-state file written by older
/// rubix-agent builds. Exposed as a constant so the boot-time
/// auto-copy and the operator migration script (see
/// `rubix/scripts/migrate-node-state-to-pg.sh`) agree on the
/// location.
const LEGACY_NODE_STATE_DB: &str = "~/.rubix/node_state.db";

/// Boot-time, best-effort copy of any surviving rows from the
/// legacy `~/.rubix/node_state.db` file into the PG `node_state`
/// table. First-writer-wins (`ON CONFLICT DO NOTHING`): rows
/// already in PG are never overwritten, so an operator who has
/// already booted the new binary and written fresh state never
/// loses it to an old snapshot.
///
/// On success the SQLite file is renamed to `*.migrated` so this
/// step is a no-op on subsequent boots. Any error short of "file
/// missing" is returned to the caller, which logs-and-continues —
/// the script under `rubix/scripts/` is the escape hatch.
async fn migrate_legacy_node_state_db(pg: &PgPool) -> Result<()> {
    let resolved = resolve_home_tilde(&PathBuf::from(LEGACY_NODE_STATE_DB));
    if !resolved.exists() {
        return Ok(());
    }
    info!(
        target: "rubix.boot.flow_runtime",
        path = %resolved.display(),
        "legacy node_state.db detected — copying surviving rows into Postgres",
    );

    // Open read-only; we never want the auto-migration to mutate
    // the legacy file beyond the post-success rename.
    let opts = SqliteConnectOptions::new()
        .filename(&resolved)
        .read_only(true)
        .disable_statement_logging();
    let sqlite_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .map_err(|e| anyhow::anyhow!("open legacy sqlite `{}`: {e}", resolved.display()))?;

    let rows = sqlx::query(
        "SELECT flow_id, node_id, key, value, version FROM node_state",
    )
    .fetch_all(&sqlite_pool)
    .await
    .map_err(|e| anyhow::anyhow!("read legacy node_state rows: {e}"))?;

    let mut copied: u64 = 0;
    let mut skipped: u64 = 0;
    let pg_sqlx = pg.sqlx();
    for row in &rows {
        let flow_id: String = row.try_get("flow_id")?;
        let node_id: String = row.try_get("node_id")?;
        let key: String = row.try_get("key")?;
        let value: Vec<u8> = row.try_get("value")?;
        let version: i64 = row.try_get("version")?;
        let res = sqlx::query(
            "INSERT INTO node_state (flow_id, node_id, key, value, version, updated_at) \
             VALUES ($1, $2, $3, $4, $5, NOW()) \
             ON CONFLICT (flow_id, node_id, key) DO NOTHING",
        )
        .bind(&flow_id)
        .bind(&node_id)
        .bind(&key)
        .bind(&value)
        .bind(version)
        .execute(pg_sqlx)
        .await
        .map_err(|e| anyhow::anyhow!("insert legacy node_state row: {e}"))?;
        if res.rows_affected() == 0 {
            skipped += 1;
        } else {
            copied += 1;
        }
    }

    // Drop the pool so the file handle is released before rename
    // (Windows-friendly; harmless on Linux).
    sqlite_pool.close().await;

    let migrated_path = {
        let mut p = resolved.clone();
        let stem = p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "node_state.db".to_owned());
        p.set_file_name(format!("{stem}.migrated"));
        p
    };
    std::fs::rename(&resolved, &migrated_path).map_err(|e| {
        anyhow::anyhow!(
            "rename legacy sqlite `{}` -> `{}`: {e}",
            resolved.display(),
            migrated_path.display(),
        )
    })?;

    info!(
        target: "rubix.boot.flow_runtime",
        copied,
        skipped,
        migrated = %migrated_path.display(),
        "legacy node_state.db migration complete",
    );
    Ok(())
}

/// Expand a `~/...` prefix against `$HOME` so the legacy
/// `~/.rubix/node_state.db` path resolves to a real file on disk.
fn resolve_home_tilde(raw: &PathBuf) -> PathBuf {
    let s = raw.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    raw.clone()
}

// ---------------------------------------------------------------------
// Bundled-schedule enumeration (generalises `boot/scheduler.rs`).
// ---------------------------------------------------------------------

/// One bundled flow whose YAML carries `trigger: schedule` +
/// `cron_expr`.
pub struct BundledSchedule {
    /// Reverse-DNS flow id from the YAML.
    pub flow_id: String,
    /// Top-level cron expression to register with the durable
    /// scheduler.
    pub cron_expr: String,
}

/// Walk `rubix_flows::BUNDLED` and return every flow whose YAML
/// declares `trigger: schedule` together with a `cron_expr`. Both
/// the legacy `boot::scheduler` tick task and the new
/// always-on mounter call this so they see an identical view of
/// the bundle. Authoring `trigger: schedule` without `cron_expr`
/// is treated as a load-time error per the existing scheduler
/// contract.
pub fn bundled_schedule_pairs() -> Result<Vec<BundledSchedule>> {
    let mut out = Vec::new();
    for (path, bytes) in walk_bundled() {
        let yaml = rubix_flows::parse_yaml(&path, &bytes)
            .map_err(|e| anyhow::anyhow!("parse bundled yaml `{path}`: {e}"))?;
        let is_schedule = yaml
            .trigger
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("schedule"))
            .unwrap_or(false);
        match (is_schedule, yaml.cron_expr.as_deref()) {
            (false, _) => continue,
            (true, None) => anyhow::bail!(
                "bundled flow `{flow}` declares `trigger: schedule` but no `cron_expr`",
                flow = yaml.id,
            ),
            (true, Some(cron_expr)) => out.push(BundledSchedule {
                flow_id: yaml.id,
                cron_expr: cron_expr.to_owned(),
            }),
        }
    }
    Ok(out)
}

fn walk_bundled() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    collect(&rubix_flows::BUNDLED, &mut out);
    out
}

fn collect(dir: &include_dir::Dir<'_>, out: &mut Vec<(String, Vec<u8>)>) {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(f) => {
                let path = f.path().to_string_lossy().into_owned();
                if path.ends_with(".yaml") || path.ends_with(".yml") {
                    out.push((path, f.contents().to_vec()));
                }
            }
            include_dir::DirEntry::Dir(sub) => collect(sub, out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subscribe_or_create_returns_independent_receivers() {
        let reg = FlowSubscriptionRegistry::new();
        let flow = FlowId::new("dev.starter.echo").expect("valid id");
        let mut rx1 = reg.subscribe_or_create(&flow).await;
        let mut rx2 = reg.subscribe_or_create(&flow).await;
        let tx = reg.sender(&flow).await;
        tx.send(FlowEvent::RunStarted {
            run: starter_flow_spi::flow::RunId::new(),
            flow: flow.clone(),
        })
        .expect("send");
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[tokio::test]
    async fn build_falls_back_to_in_memory_without_database_url() {
        let cfg = FlowRuntimeConfig::default();
        let rt = build(None, &cfg).await.expect("build");
        // Surface a smoke check: the in-memory store accepts a put.
        use starter_flow_spi::node::NodeId;
        let key = starter_flow_spi::state::NodeStateKey::new(
            FlowId::new("dev.starter.echo").unwrap(),
            NodeId::new("dev.starter.counter").unwrap(),
            "count",
        )
        .unwrap();
        rt.state_store.put(&key, b"1".to_vec()).await.expect("put");
    }
}
