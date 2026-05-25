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
//!    (`DOCS/flow/scope/node-state.md`). When `RUBIX_DATABASE_URL` is
//!    set *and* a SQLite `state_db_path` is configured under
//!    `[flow_runtime]` we open the SQLite-backed
//!    [`SqliteNodeStateStore`] over `~/.rubix/node_state.db`; otherwise
//!    we fall back to [`InMemoryNodeStateStore`] so a laptop boot
//!    without Postgres still has a working seam.
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
use sqlx::ConnectOptions;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use starter_flow::state::in_memory::InMemoryNodeStateStore;
use starter_flow_spi::flow::{FlowEvent, FlowId};
use starter_flow_spi::state::NodeStateStore;
use starter_store_sqlite::flow::node_state::SqliteNodeStateStore;
use starter_store_sqlite::flow::FLOW_MIGRATION_SOURCE;
use starter_store_sqlite::migrate;
use starter_store_sqlite::pool::Pool as SqlitePool;

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
        if let Some(tx) = self.inner.read().await.get(flow_id) {
            return tx.subscribe();
        }
        // Slow path: install a fresh sender. The race window is
        // benign — a second concurrent caller may have inserted
        // first; we re-check under the write lock and reuse.
        let mut map = self.inner.write().await;
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
        if let Some(tx) = self.inner.read().await.get(flow_id) {
            return tx.clone();
        }
        let mut map = self.inner.write().await;
        map.entry(flow_id.clone())
            .or_insert_with(|| broadcast::channel::<FlowEvent>(BROADCAST_CAPACITY).0)
            .clone()
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
/// based on `(database_url, cfg.state_db_path)` and create an empty
/// [`FlowSubscriptionRegistry`].
///
/// Selection rule:
///
/// - `database_url = Some(_)` *and* `cfg.state_db_path = Some(_)` —
///   open a SQLite pool at the path, apply [`FLOW_MIGRATION_SOURCE`],
///   and wrap it in [`SqliteNodeStateStore`]. The path's parent
///   directory is created on demand so a fresh `~/.rubix/` is
///   self-healing.
/// - Otherwise — fall back to [`InMemoryNodeStateStore`] and log so
///   the operator can see they're on the volatile path.
pub async fn build(
    database_url: Option<&str>,
    cfg: &FlowRuntimeConfig,
) -> Result<FlowRuntime> {
    let state_store: Arc<dyn NodeStateStore> = match (database_url, cfg.state_db_path.as_ref()) {
        (Some(_), Some(path)) => {
            let resolved = resolve_state_db_path(path);
            if let Some(parent) = resolved.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        anyhow::anyhow!(
                            "create node_state parent dir `{}`: {e}",
                            parent.display()
                        )
                    })?;
                }
            }
            let opts = SqliteConnectOptions::new()
                .filename(&resolved)
                .create_if_missing(true)
                .disable_statement_logging();
            let sqlx_pool = SqlitePoolOptions::new()
                .max_connections(4)
                .connect_with(opts)
                .await
                .map_err(|e| anyhow::anyhow!("open node_state sqlite `{}`: {e}", resolved.display()))?;
            let pool = SqlitePool::from_sqlx(sqlx_pool);
            migrate::migrate(&pool)
                .with_source(FLOW_MIGRATION_SOURCE)
                .run()
                .await
                .map_err(|e| anyhow::anyhow!("apply flow migrations to node_state db: {e}"))?;
            info!(
                target: "rubix.boot.flow_runtime",
                path = %resolved.display(),
                "NodeStateStore: SQLite (durable)",
            );
            Arc::new(SqliteNodeStateStore::new(pool))
        }
        _ => {
            warn!(
                target: "rubix.boot.flow_runtime",
                "NodeStateStore: in-memory (volatile) — set RUBIX_DATABASE_URL and \
                 [flow_runtime].state_db_path to opt into durable per-node state",
            );
            Arc::new(InMemoryNodeStateStore::new())
        }
    };
    Ok(FlowRuntime {
        subscriptions: Arc::new(FlowSubscriptionRegistry::new()),
        state_store,
    })
}

/// Expand a `~/...` prefix against `$HOME` so the default
/// `~/.rubix/node_state.db` Just Works without a leaked tilde in
/// the SQLite filename.
fn resolve_state_db_path(raw: &PathBuf) -> PathBuf {
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
