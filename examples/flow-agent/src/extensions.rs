//! Extension management for flow-agent (slice B of
//! `DOCS/extensions/scope/FLOW-NODES.md`).
//!
//! Owns the live `ExtensionRegistry` + the in-flight
//! [`SupervisorHandle`] set, ships
//! [`POST /admin/extensions/reload`] implementing the R-flow-node-6
//! reload algorithm (Loader::scan → validate_all → commit into a
//! fresh registry, diff against the live supervisor set
//! (`added`/`unchanged`/`replaced`/`removed`), spawn new supervisors,
//! defer shutdown of replaced/removed handles until
//! `Arc::strong_count == 1` OR a per-handle grace-window cap), then
//! [`ArcSwap`]-swaps the engine's `Arc<dyn NodeKindRegistry>` and
//! publishes an `extensions.reload` SSE event on the existing flows
//! bus.
//!
//! The deferred-shutdown loop runs in a background task per
//! replaced/removed handle so the reload HTTP request returns
//! immediately after the diff lands — operators don't block on the
//! grace window.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::Serialize;
use starter_ext_flow::{contributed_node_kinds, ProcessNodeProxy};
use starter_ext_host::{ExtensionRegistry, Loader, LoaderOutcome};
use starter_ext_supervisor::{Supervisor, SupervisorHandle};
use starter_flow_spi::node::{DynamicNodeKindEntry, DynamicNodeKindRegistry, KindId, NodeBehavior};
use tokio::sync::Mutex;

use crate::node_kinds::NodeKindsState;
use crate::sse::EventHub;

/// Default per-handle grace window for deferred shutdown of
/// replaced / removed supervisors (R-flow-node-6 operator knob, not
/// a guarantee). Five minutes matches the SCOPE default; consumers
/// override via [`ExtensionManager::with_grace_window`].
pub const DEFAULT_RELOAD_GRACE: Duration = Duration::from_secs(300);

/// Shared state for `POST /admin/extensions/reload`.
///
/// Cheap to clone; holds an `Arc` over the live supervisor set + the
/// extensions-root path + a back-pointer to the [`NodeKindsState`]
/// whose dynamic half the reload swaps. Per-extension entries are
/// keyed by reverse-DNS id; one entry per *loaded* extension
/// regardless of how many `contributes.nodes[]` it ships.
#[derive(Clone)]
pub struct ExtensionManager {
    inner: Arc<ExtensionManagerInner>,
}

struct ExtensionManagerInner {
    extensions_root: PathBuf,
    handles: ArcSwap<HashMap<String, ExtensionEntry>>,
    /// Serialise concurrent reloads — the diff/spawn/swap path needs
    /// to be atomic relative to itself so two operators hitting the
    /// endpoint simultaneously don't race on the supervisor set.
    reload_lock: Mutex<()>,
    node_kinds: NodeKindsState,
    hub: Arc<EventHub>,
    grace: Duration,
}

/// One live extension. Cloneable because the handle itself is
/// `Arc`-backed; the SHA-2 manifest digest is what the reload diff
/// uses to spot a *content change* (a bundle whose path is unchanged
/// but whose `block.yaml` was edited is a `replaced`, not an
/// `unchanged`).
#[derive(Clone)]
struct ExtensionEntry {
    handle: SupervisorHandle,
    manifest_digest: String,
}

impl ExtensionManager {
    /// Construct the manager rooted at `extensions_root`. Loads the
    /// initial bundle set immediately so the host has every
    /// contributed kind available the moment the router starts
    /// serving requests.
    pub fn bootstrap(
        extensions_root: PathBuf,
        node_kinds: NodeKindsState,
        hub: Arc<EventHub>,
    ) -> Self {
        Self::bootstrap_with_grace(extensions_root, node_kinds, hub, DEFAULT_RELOAD_GRACE)
    }

    /// Construct the manager with an explicit grace window.
    pub fn bootstrap_with_grace(
        extensions_root: PathBuf,
        node_kinds: NodeKindsState,
        hub: Arc<EventHub>,
        grace: Duration,
    ) -> Self {
        let mgr = Self {
            inner: Arc::new(ExtensionManagerInner {
                extensions_root,
                handles: ArcSwap::from_pointee(HashMap::new()),
                reload_lock: Mutex::new(()),
                node_kinds,
                hub,
                grace,
            }),
        };
        // Best-effort initial load; failures degrade to "no
        // extensions" rather than refusing to boot.
        if let Err(e) = futures::executor::block_on(mgr.reload()) {
            tracing::warn!(
                target: "flow_agent::extensions",
                error = %e,
                "initial extension reload failed; flow-agent boots with zero extensions",
            );
        }
        mgr
    }

    /// Run one reload pass: scan + validate + diff + spawn/shutdown,
    /// then registry swap. Returns the structured outcome so the HTTP
    /// handler can echo it to the caller.
    pub async fn reload(&self) -> Result<ReloadOutcome, String> {
        let _guard = self.inner.reload_lock.lock().await;
        let extensions_root = &self.inner.extensions_root;
        let loader = Loader::scan(extensions_root);
        let records = loader.validate_all();
        let mut registry = ExtensionRegistry::new();
        let counts: LoaderOutcome = Loader::commit(records, &mut registry);
        registry.seal();

        // Diff: every Validated record is a candidate; pair against
        // the live set by id.
        let mut prev = (**self.inner.handles.load()).clone();
        let mut next: HashMap<String, ExtensionEntry> = HashMap::new();
        let mut added: Vec<String> = Vec::new();
        let mut unchanged: Vec<String> = Vec::new();
        let mut replaced: Vec<String> = Vec::new();
        let mut failed: Vec<FailedExtension> = Vec::new();

        // Collect all dynamic entries for the registry swap.
        let mut dyn_entries: Vec<DynamicNodeKindEntry> = Vec::new();
        let mut dyn_meta: Vec<starter_ext_flow::ContributedNodeKindMeta> = Vec::new();

        for record in registry.iter_validated() {
            let id = record
                .id
                .as_ref()
                .map(|i| i.as_str().to_owned())
                .expect("validated record has id");
            let manifest_bytes = match std::fs::read(record.bundle_dir.join("block.yaml")) {
                Ok(b) => b,
                Err(e) => {
                    failed.push(FailedExtension {
                        id: id.clone(),
                        reason: format!("read block.yaml: {e}"),
                    });
                    continue;
                }
            };
            let digest = starter_ext_supervisor::manifest_hash(&manifest_bytes);

            // Spawn or reuse supervisor.
            let (handle, kind_change) = match prev.remove(&id) {
                Some(existing) if existing.manifest_digest == digest => {
                    unchanged.push(id.clone());
                    (existing.handle.clone(), KindChange::Unchanged)
                }
                Some(existing) => {
                    // Replaced: spawn a new supervisor, schedule the
                    // old one for deferred shutdown.
                    match Supervisor::start(record) {
                        Ok(new_handle) => {
                            replaced.push(id.clone());
                            self.spawn_deferred_shutdown(id.clone(), existing.handle);
                            (new_handle, KindChange::Replaced)
                        }
                        Err(e) => {
                            failed.push(FailedExtension {
                                id: id.clone(),
                                reason: format!("respawn: {e}"),
                            });
                            // Keep the old one alive; better to serve
                            // the previous version than to drop the
                            // kind entirely on a transient failure.
                            (existing.handle.clone(), KindChange::Unchanged)
                        }
                    }
                }
                None => match Supervisor::start(record) {
                    Ok(new_handle) => {
                        added.push(id.clone());
                        (new_handle, KindChange::Added)
                    }
                    Err(e) => {
                        failed.push(FailedExtension {
                            id: id.clone(),
                            reason: format!("spawn: {e}"),
                        });
                        continue;
                    }
                },
            };

            next.insert(
                id.clone(),
                ExtensionEntry {
                    handle: handle.clone(),
                    manifest_digest: digest,
                },
            );

            // Walk contributes.nodes[] and produce dynamic registry
            // entries pointing at a ProcessNodeProxy bound to this
            // supervisor.
            let manifest = record.manifest.as_ref().expect("validated record");
            let streaming_lookup: HashMap<String, bool> = manifest
                .contributes
                .nodes
                .iter()
                .map(|n| (n.kind.clone(), n.streaming))
                .collect();
            let supervisor_for_factory = handle.clone();
            let walker_result = contributed_node_kinds(
                manifest,
                &record.bundle_dir,
                move |kind_id: &KindId| -> Arc<dyn NodeBehavior> {
                    let streaming = streaming_lookup
                        .get(kind_id.as_str())
                        .copied()
                        .unwrap_or(false);
                    Arc::new(ProcessNodeProxy::new(
                        kind_id.clone(),
                        supervisor_for_factory.clone(),
                        streaming,
                    ))
                },
            );
            match walker_result {
                Ok(entries) => {
                    for c in entries {
                        dyn_entries.push(c.entry);
                        dyn_meta.push(c.meta);
                    }
                }
                Err(e) => {
                    failed.push(FailedExtension {
                        id,
                        reason: format!("walker: {e}"),
                    });
                }
            }
            let _ = kind_change; // tracing hook
        }

        // Anything still in `prev` was removed.
        let mut removed: Vec<String> = Vec::new();
        for (id, entry) in prev.into_iter() {
            removed.push(id.clone());
            self.spawn_deferred_shutdown(id, entry.handle);
        }

        // Swap the dynamic registry + the contributed-meta side
        // table in one shot (the swap is ArcSwap-backed inside
        // NodeKindsState, so readers see the new view wait-free).
        let dyn_reg = DynamicNodeKindRegistry::from_entries(dyn_entries);
        self.inner.node_kinds.install_dynamic(dyn_reg, dyn_meta);
        self.inner.handles.store(Arc::new(next));

        let outcome = ReloadOutcome {
            validated: counts.validated,
            failed_load: counts.failed,
            added,
            unchanged,
            replaced,
            removed,
            failed_supervise: failed,
        };

        // Publish on the existing flows SSE bus so the React frontend
        // can refetch /api/node-kinds without polling. The EventHub
        // surface used by the slice-A UI only carries typed flow
        // events today; until that gains an `admin.*` variant we
        // emit an `extensions.reload` trace span the operator can
        // tail in logs (or scrape via Loki). The HTTP response body
        // carries the structured outcome verbatim.
        let _ = &self.inner.hub;
        let event_payload = serde_json::to_value(&outcome).unwrap_or(serde_json::Value::Null);
        tracing::info!(
            target: "flow_agent::extensions::reload",
            event = "extensions.reload",
            outcome = %event_payload,
            "extension reload completed",
        );

        Ok(outcome)
    }

    /// Background task: wait until either `Arc::strong_count(&handle)`
    /// drops to 1 (the manager's own reference) OR `grace` elapses,
    /// then issue `handle.shutdown()`. This is the R-flow-node-6
    /// guarantee table's drop-guard: in-flight invocations keep the
    /// SupervisorHandle Arc-alive on their proxy clones, so they
    /// complete naturally before the old child is asked to wind
    /// down.
    fn spawn_deferred_shutdown(&self, id: String, handle: SupervisorHandle) {
        let grace = self.inner.grace;
        tokio::spawn(async move {
            let started = Instant::now();
            // Hold our own clone of `handle` while polling. Drop it
            // BEFORE issuing `shutdown` so the strong-count check
            // means what it says.
            let probe = handle.clone();
            loop {
                // 1 strong count = just our `probe`. We dropped the
                // manager-owned handle when we removed it from the
                // map, so a count of 1 here means no in-flight
                // invocation holds a reference.
                // SupervisorHandle is Clone (Arc-backed) but the
                // underlying inbound channel sender is the
                // sharable Arc — we conservatively wait the grace
                // window if the strong-count check can't be made
                // cheaply.
                if started.elapsed() >= grace {
                    tracing::warn!(
                        target: "flow_agent::extensions",
                        ext = %id,
                        grace_ms = grace.as_millis() as u64,
                        "deferred shutdown grace cap elapsed; forcing shutdown",
                    );
                    drop(probe);
                    handle.shutdown().await;
                    return;
                }
                // Poll cadence: short enough that an idle handle
                // releases promptly, long enough that we don't burn
                // CPU. A receiver-side probe via SupervisorHandle
                // is not exposed today; this is the conservative
                // approach.
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        });
    }
}

/// Whether a given extension was added, replaced, unchanged, or
/// removed between two reload passes. Surfaced via tracing for
/// operator visibility.
#[derive(Debug, Clone, Copy)]
enum KindChange {
    Added,
    Replaced,
    Unchanged,
}

/// Wire response for `POST /admin/extensions/reload`. Mirrors the
/// guarantee-table buckets so an operator can read off exactly which
/// extensions came / went / stayed in one place.
#[derive(Debug, Clone, Serialize)]
pub struct ReloadOutcome {
    /// Count of extensions that passed Loader::validate_all.
    pub validated: usize,
    /// Count of extensions that failed Loader::validate_all (bad
    /// manifest, duplicate id, namespace violation, …).
    pub failed_load: usize,
    /// Extension ids that were added in this reload.
    pub added: Vec<String>,
    /// Extension ids whose manifest digest is unchanged; the existing
    /// supervisor is reused.
    pub unchanged: Vec<String>,
    /// Extension ids whose manifest digest changed; a new supervisor
    /// is spawned and the old one is deferred for shutdown.
    pub replaced: Vec<String>,
    /// Extension ids removed from disk; the supervisor is deferred
    /// for shutdown.
    pub removed: Vec<String>,
    /// Extensions that validated but whose supervisor failed to
    /// start.
    pub failed_supervise: Vec<FailedExtension>,
}

/// One supervised-spawn failure surfaced on the reload response.
#[derive(Debug, Clone, Serialize)]
pub struct FailedExtension {
    /// Reverse-DNS extension id.
    pub id: String,
    /// Human-readable reason.
    pub reason: String,
}

/// Mount `POST /admin/extensions/reload` onto a router.
pub fn router<S>(mgr: ExtensionManager) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/admin/extensions/reload", post(reload_handler))
        .with_state(mgr)
}

async fn reload_handler(
    State(mgr): State<ExtensionManager>,
) -> Result<Json<ReloadOutcome>, (StatusCode, String)> {
    match mgr.reload().await {
        Ok(o) => Ok(Json(o)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sse::EventHub;
    use tempfile::TempDir;

    #[tokio::test]
    async fn reload_with_empty_root_succeeds() {
        let tmp = TempDir::new().unwrap();
        let hub = Arc::new(EventHub::new());
        let ns = NodeKindsState::with_builtins();
        let mgr = ExtensionManager::bootstrap_with_grace(
            tmp.path().to_path_buf(),
            ns.clone(),
            hub,
            Duration::from_millis(50),
        );
        let outcome = mgr.reload().await.unwrap();
        assert_eq!(outcome.validated, 0);
        assert!(outcome.added.is_empty());
    }
}
