//! [`ExtensionAdmin`] — shared state for the admin routes.
//!
//! One cheap-to-clone handle the router and every handler share. It
//! wraps:
//!
//! - The sealed [`ExtensionRegistry`] returned from
//!   `Loader::commit`. Immutable for the host's lifetime.
//! - A map of currently-running [`SupervisorHandle`]s keyed by
//!   extension id. Mutable: disable removes an entry (after sending
//!   shutdown), enable inserts one (after re-spawning via the factory).
//! - An [`EnablementStore`] for persistence.
//! - A [`SupervisorFactory`] for spawning on enable.
//! - The on-disk ETag cache used by `GET /extensions/<id>/ui/*`.
//!
//! The struct is cheap to `Clone` because the heavy state is behind
//! `Arc`. Handlers take a `State<ExtensionAdmin>` and pull what they
//! need.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use starter_ext_host::ExtensionRegistry;
use starter_ext_metrics::MetricsRegistry;
use starter_ext_spi::{ExtensionId, Manifest, RuntimeKind};
use starter_ext_supervisor::SupervisorHandle;

use crate::audit::{AuditSink, NoopAuditSink};
use crate::cleanup::{
    CleanupItem, CleanupProvider, EnablementRowProvider, I18nCacheProvider, PostInstallHook,
    UiCacheProvider,
};
use crate::etag::EtagCache;
use crate::factory::{DefaultSupervisorFactory, DynFactory};
use crate::store::{EnablementStore, InMemoryEnablementStore};

/// Cheap-to-clone shared state. See module docs.
#[derive(Clone)]
pub struct ExtensionAdmin {
    inner: Arc<Inner>,
}

struct Inner {
    registry: Arc<ExtensionRegistry>,
    supervisors: RwLock<HashMap<String, SupervisorHandle>>,
    store: Arc<dyn EnablementStore>,
    factory: DynFactory,
    etag_cache: Arc<EtagCache>,
    /// Registered cleanup providers, in run order. The built-in
    /// enablement-row + UI/i18n-cache providers are prepended at
    /// `build()`; consumer-supplied providers (rubix's warehouse + skill
    /// reclaimers) follow.
    cleanup_providers: Vec<Arc<dyn CleanupProvider>>,
    /// Optional consumer-supplied step run by the install handler right
    /// after a bundle is staged + validated (rubix uses it to create the
    /// bundle's warehouse tables immediately, instead of waiting for the
    /// next boot's DDL pass). `None` on a `TestApp` or the generic host.
    post_install_hook: Option<Arc<dyn PostInstallHook>>,
    /// Ids installed during this process run that are not yet live in the
    /// sealed registry — they surface on next boot. Surfaced as
    /// `restart_required` on the list projection so the UI can badge them.
    pending_restart: RwLock<HashMap<String, PendingInstall>>,
    /// Ids purged (`DELETE …?purge=true`) during this process run. The
    /// sealed registry still carries their record until the next boot, so
    /// the row keeps reporting `validated/enabled` even though their
    /// persisted state (kinds, enablement row, owned tables) is already
    /// gone — which reads as "still installed" to an operator. Marking the
    /// id here lets the list/detail projection report `uninstalled: true`
    /// (+ `restart_required: true`) so the UI can show it as a dead/stale
    /// row pending a restart to clear. Cleared if the id is re-enabled.
    uninstalled: RwLock<HashSet<String>>,
    /// Per-extension counter registry. Shared with the transport adapters
    /// (they bump it) and read by `GET /extensions/<id>/metrics`. Defaults
    /// to a fresh empty registry so a `TestApp` that does not wire the
    /// adapters still serves all-zero counters.
    metrics: MetricsRegistry,
    /// Consumer-supplied lifecycle audit sink. The enable/disable/install/
    /// uninstall handlers notify it with the acting principal after a
    /// successful mutation. Defaults to [`NoopAuditSink`] when unset.
    audit_sink: Arc<dyn AuditSink>,
    worker_states: Option<WorkerStatesFn>,
    /// On-disk root for installed (uploaded-tarball) bundles. The
    /// install handler unpacks here and the uninstall handler removes
    /// directories under it. Under the installed-only model this is
    /// the *only* place extension bundles live at runtime. Endpoints
    /// return HTTP 503 when this is unset so a `TestApp` that doesn't
    /// wire it stays functional for the toggle-only surface.
    installs_dir: Option<PathBuf>,
}

/// Closure shape the admin route calls when rendering
/// `GET /extensions/<id>`'s `workers:` field.
///
/// This is the seam between `starter-ext-server` and any periodic-
/// worker adapter (notably `starter-ext-workers`, Adapter Phase 7).
/// Keeping it as a `Fn(&ExtensionId) -> Vec<Value>` means the admin
/// crate does not depend on the workers adapter; the consumer wires
/// its own scheduler handle in.
pub type WorkerStatesFn =
    Arc<dyn Fn(&ExtensionId) -> Vec<serde_json::Value> + Send + Sync + 'static>;

/// A lightweight summary of an extension installed during this process run
/// but not yet live in the sealed registry (it surfaces on next boot).
/// Captured from the validated install record so the list projection can
/// render a badge-able row without re-reading the bundle.
#[derive(Debug, Clone)]
pub(crate) struct PendingInstall {
    pub version: Option<String>,
    pub display_name: Option<String>,
    pub runtime_kind: Option<RuntimeKind>,
}

impl ExtensionAdmin {
    /// Start building from a sealed registry.
    pub fn builder(registry: Arc<ExtensionRegistry>) -> ExtensionAdminBuilder {
        ExtensionAdminBuilder {
            registry,
            supervisors: HashMap::new(),
            store: None,
            factory: None,
            metrics: None,
            worker_states: None,
            installs_dir: None,
            cleanup_providers: Vec::new(),
            post_install_hook: None,
            audit_sink: None,
        }
    }

    /// Read the underlying registry. Adapters / tests may need direct
    /// access to enumerate records.
    pub fn registry(&self) -> &ExtensionRegistry {
        &self.inner.registry
    }

    /// Look up a live supervisor handle by id.
    pub(crate) fn supervisor(&self, id: &ExtensionId) -> Option<SupervisorHandle> {
        self.inner
            .supervisors
            .read()
            .expect("supervisor map poisoned")
            .get(id.as_str())
            .cloned()
    }

    /// Replace (or remove) the live supervisor handle for an id.
    /// Returns the previous handle, if any.
    pub(crate) fn replace_supervisor(
        &self,
        id: &ExtensionId,
        new: Option<SupervisorHandle>,
    ) -> Option<SupervisorHandle> {
        let mut map = self
            .inner
            .supervisors
            .write()
            .expect("supervisor map poisoned");
        match new {
            Some(h) => map.insert(id.as_str().to_string(), h),
            None => map.remove(id.as_str()),
        }
    }

    /// Shut down every live supervisor (`SIGTERM` → grace → `SIGKILL`) and
    /// clear the map. Called by the host at process exit so no extension child
    /// outlives the host. Idempotent — a second call finds an empty map and is a
    /// no-op. Builtin/wasm records have no handle and are unaffected.
    pub async fn shutdown_all(&self) {
        // Drain the handles out under the lock, then await their shutdowns
        // outside it (the lock is sync; awaiting while holding it would be a
        // deadlock risk and blocks concurrent reads).
        let handles: Vec<SupervisorHandle> = {
            let mut map = self
                .inner
                .supervisors
                .write()
                .expect("supervisor map poisoned");
            map.drain().map(|(_, h)| h).collect()
        };
        for handle in handles {
            handle.shutdown().await;
        }
    }

    pub(crate) fn store(&self) -> &dyn EnablementStore {
        &*self.inner.store
    }

    pub(crate) fn factory(&self) -> &DynFactory {
        &self.inner.factory
    }

    /// The lifecycle audit sink. Always present (defaults to the no-op sink);
    /// the lifecycle handlers call it after a successful mutation.
    pub(crate) fn audit_sink(&self) -> &Arc<dyn AuditSink> {
        &self.inner.audit_sink
    }

    pub(crate) fn etag_cache(&self) -> &EtagCache {
        &self.inner.etag_cache
    }

    /// The shared per-extension metrics registry. The consumer hands the
    /// same handle to the transport adapters at wiring time so their
    /// counter bumps land in the registry `GET /extensions/<id>/metrics`
    /// reads.
    pub fn metrics(&self) -> &MetricsRegistry {
        &self.inner.metrics
    }

    /// Render the worker-state field for `GET /extensions/<id>`.
    /// Returns an empty vector when no provider is wired — the JSON
    /// response still includes a `workers: []` field, which is the
    /// truthful shape for hosts that did not opt into the periodic-
    /// worker adapter.
    /// On-disk root for installed (uploaded-tarball) bundles, when
    /// wired. Used by the install / uninstall endpoints (Phase D.1).
    pub(crate) fn installs_dir(&self) -> Option<&std::path::Path> {
        self.inner.installs_dir.as_deref()
    }

    /// The consumer-supplied post-install hook, if one was wired. The
    /// install handler runs it after a successful install.
    pub(crate) fn post_install_hook(&self) -> Option<&Arc<dyn PostInstallHook>> {
        self.inner.post_install_hook.as_ref()
    }

    pub(crate) fn worker_states(&self, id: &ExtensionId) -> Vec<serde_json::Value> {
        match &self.inner.worker_states {
            Some(f) => f(id),
            None => Vec::new(),
        }
    }

    /// Record an extension installed during this run as pending a restart
    /// (it surfaces on next boot). Surfaced via [`Self::pending_rows`].
    pub(crate) fn mark_pending_restart(&self, id: &str, pending: PendingInstall) {
        self.inner
            .pending_restart
            .write()
            .expect("pending_restart poisoned")
            .insert(id.to_owned(), pending);
    }

    /// Drop an id from the pending-restart set (e.g. on purge).
    pub(crate) fn clear_pending_restart(&self, id: &str) {
        self.inner
            .pending_restart
            .write()
            .expect("pending_restart poisoned")
            .remove(id);
    }

    /// Is this id awaiting a restart to go live?
    pub(crate) fn is_pending_restart(&self, id: &str) -> bool {
        self.inner
            .pending_restart
            .read()
            .expect("pending_restart poisoned")
            .contains_key(id)
    }

    /// Mark an id as purged this run (`DELETE …?purge=true`). Its sealed
    /// registry record lingers until the next boot, so the projection
    /// reports it as `uninstalled` + `restart_required` until then.
    pub(crate) fn mark_uninstalled(&self, id: &str) {
        self.inner
            .uninstalled
            .write()
            .expect("uninstalled poisoned")
            .insert(id.to_owned());
    }

    /// Drop an id from the purged set (e.g. on re-enable, so a re-enabled
    /// extension stops reporting as dead/stale).
    pub(crate) fn clear_uninstalled(&self, id: &str) {
        self.inner
            .uninstalled
            .write()
            .expect("uninstalled poisoned")
            .remove(id);
    }

    /// Was this id purged this run and is its (now-stale) record still in
    /// the sealed registry, awaiting a restart to clear?
    pub(crate) fn is_uninstalled(&self, id: &str) -> bool {
        self.inner
            .uninstalled
            .read()
            .expect("uninstalled poisoned")
            .contains(id)
    }

    /// Snapshot of the pending-restart ids and their captured summaries —
    /// used by the list projection to append rows for freshly-installed
    /// extensions not yet present in the sealed registry.
    pub(crate) fn pending_rows(&self) -> Vec<(String, PendingInstall)> {
        self.inner
            .pending_restart
            .read()
            .expect("pending_restart poisoned")
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Run every registered cleanup provider's `discover` and concatenate
    /// the results — the dry-run manifest for `GET /extensions/<id>/cleanup`.
    pub(crate) async fn discover_cleanup(
        &self,
        id: &ExtensionId,
        manifest: Option<&Manifest>,
    ) -> Vec<CleanupItem> {
        let mut out = Vec::new();
        for provider in &self.inner.cleanup_providers {
            out.extend(provider.discover(id, manifest).await);
        }
        out
    }

    /// Run every registered cleanup provider's `purge`, returning the items
    /// actually removed. Each provider discovers its own items, purges
    /// them, and every removed item is logged with
    /// `target: "starter_ext_server::cleanup"` and the caller principal. A
    /// single provider failing logs a warning and never aborts the others —
    /// purge is best-effort and idempotent.
    pub(crate) async fn purge_cleanup(
        &self,
        id: &ExtensionId,
        manifest: Option<&Manifest>,
        principal: &str,
    ) -> Vec<CleanupItem> {
        let mut removed = Vec::new();
        for provider in &self.inner.cleanup_providers {
            let items = provider.discover(id, manifest).await;
            if items.is_empty() {
                continue;
            }
            match provider.purge(id, &items).await {
                Ok(()) => {
                    for item in &items {
                        tracing::info!(
                            target: "starter_ext_server::cleanup",
                            id = %id.as_str(),
                            principal = %principal,
                            kind = ?item.kind,
                            label = %item.label,
                            bytes = item.bytes,
                            "purged cleanup item",
                        );
                    }
                    removed.extend(items);
                }
                Err(e) => {
                    tracing::warn!(
                        target: "starter_ext_server::cleanup",
                        id = %id.as_str(),
                        principal = %principal,
                        err = %e,
                        "cleanup provider purge failed",
                    );
                }
            }
        }
        removed
    }
}

/// Fluent builder. `registry` is the only required input; the rest
/// fall back to sensible defaults so a `TestApp` can construct one
/// with `.build()` and no further setup.
pub struct ExtensionAdminBuilder {
    registry: Arc<ExtensionRegistry>,
    supervisors: HashMap<String, SupervisorHandle>,
    store: Option<Arc<dyn EnablementStore>>,
    factory: Option<DynFactory>,
    metrics: Option<MetricsRegistry>,
    worker_states: Option<WorkerStatesFn>,
    installs_dir: Option<PathBuf>,
    cleanup_providers: Vec<Arc<dyn CleanupProvider>>,
    post_install_hook: Option<Arc<dyn PostInstallHook>>,
    audit_sink: Option<Arc<dyn AuditSink>>,
}

impl ExtensionAdminBuilder {
    /// Pre-populate the supervisor map. Called by the host immediately
    /// after `Loader::commit` returns, once supervisors have been
    /// spawned for every initially-enabled process record.
    pub fn with_supervisors(mut self, supervisors: HashMap<String, SupervisorHandle>) -> Self {
        self.supervisors = supervisors;
        self
    }

    /// Wire a custom persistence backend. Defaults to
    /// [`InMemoryEnablementStore`] when unset.
    pub fn with_enablement_store(mut self, store: Arc<dyn EnablementStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Wire a custom supervisor factory. Defaults to
    /// [`DefaultSupervisorFactory`] when unset.
    pub fn with_supervisor_factory(mut self, factory: DynFactory) -> Self {
        self.factory = Some(factory);
        self
    }

    /// Wire the shared per-extension [`MetricsRegistry`]. The same handle
    /// must be passed to the transport adapters (mcp / REST router /
    /// workers scheduler) so their counter bumps are visible to
    /// `GET /extensions/<id>/metrics`. Defaults to a fresh empty registry
    /// (all-zero counters) when unset.
    pub fn with_metrics(mut self, metrics: MetricsRegistry) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Wire a periodic-worker state provider — typically the handle
    /// returned by `starter_ext_workers::WorkersScheduler::start`,
    /// adapted into a closure that serialises each `WorkerState`
    /// into JSON. When unset, the admin route's `workers:` field is
    /// always `[]` (truthful for hosts that did not opt into the
    /// workers adapter).
    pub fn with_worker_states_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&ExtensionId) -> Vec<serde_json::Value> + Send + Sync + 'static,
    {
        self.worker_states = Some(Arc::new(f));
        self
    }

    /// Wire the on-disk root for **installed** (uploaded-tarball)
    /// bundles. The install handler unpacks tarballs into this
    /// directory; the uninstall handler removes them from here.
    /// Dev source trees live elsewhere and are loaded read-only —
    /// the uninstall handler never deletes those regardless of this
    /// setting. When unset, install / uninstall both return HTTP 503.
    pub fn with_installs_dir(mut self, dir: PathBuf) -> Self {
        self.installs_dir = Some(dir);
        self
    }

    /// Register a [`CleanupProvider`]. Consumer-supplied providers (rubix's
    /// warehouse-table + skill reclaimers) are appended after the built-in
    /// enablement-row + UI/i18n-cache providers, which auto-register at
    /// [`Self::build`].
    pub fn with_cleanup_provider(mut self, provider: Arc<dyn CleanupProvider>) -> Self {
        self.cleanup_providers.push(provider);
        self
    }

    /// Register the [`PostInstallHook`] the install handler runs after a
    /// bundle is validated (rubix wires its warehouse-table creator here so
    /// a freshly-installed bundle's tables exist immediately, rather than
    /// only after the next boot's DDL pass).
    pub fn with_post_install_hook(mut self, hook: Arc<dyn PostInstallHook>) -> Self {
        self.post_install_hook = Some(hook);
        self
    }

    /// Wire a lifecycle [`AuditSink`]. The enable/disable/install/uninstall
    /// handlers notify it with the acting principal after a successful
    /// mutation. Defaults to [`NoopAuditSink`] when unset, so a host that does
    /// not keep an audit ledger needs no wiring (nexus wires its `nexus_changes`
    /// recorder here).
    pub fn with_audit_sink(mut self, sink: Arc<dyn AuditSink>) -> Self {
        self.audit_sink = Some(sink);
        self
    }

    /// Materialise the [`ExtensionAdmin`].
    pub fn build(self) -> ExtensionAdmin {
        let registry = self.registry;
        let store = self
            .store
            .unwrap_or_else(|| Arc::new(InMemoryEnablementStore::new()));
        let etag_cache = Arc::new(EtagCache::new());

        // Built-in providers auto-register first; they need no rubix
        // knowledge. Consumer-supplied providers follow.
        let mut cleanup_providers: Vec<Arc<dyn CleanupProvider>> = vec![
            Arc::new(EnablementRowProvider::new(store.clone())),
            Arc::new(UiCacheProvider::new(registry.clone(), etag_cache.clone())),
            Arc::new(I18nCacheProvider::new(registry.clone(), etag_cache.clone())),
        ];
        cleanup_providers.extend(self.cleanup_providers);

        ExtensionAdmin {
            inner: Arc::new(Inner {
                registry,
                supervisors: RwLock::new(self.supervisors),
                store,
                factory: self
                    .factory
                    .unwrap_or_else(|| Arc::new(DefaultSupervisorFactory::default())),
                etag_cache,
                cleanup_providers,
                pending_restart: RwLock::new(HashMap::new()),
                uninstalled: RwLock::new(HashSet::new()),
                metrics: self.metrics.unwrap_or_default(),
                audit_sink: self
                    .audit_sink
                    .unwrap_or_else(|| Arc::new(NoopAuditSink)),
                worker_states: self.worker_states,
                installs_dir: self.installs_dir,
                post_install_hook: self.post_install_hook,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_admin() -> ExtensionAdmin {
        let mut reg = ExtensionRegistry::new();
        reg.seal();
        ExtensionAdmin::builder(Arc::new(reg)).build()
    }

    #[test]
    fn uninstalled_mark_is_tracked_and_clearable() {
        let admin = test_admin();
        let id = "com.acme.purgeme";

        // Default: nothing is marked uninstalled.
        assert!(!admin.is_uninstalled(id));

        // Purge marks it — the lingering sealed-registry record now reads
        // as dead/stale to the list/detail projection.
        admin.mark_uninstalled(id);
        assert!(admin.is_uninstalled(id));

        // Re-enable (or any explicit clear) drops the mark, so a brought-
        // back extension stops reporting as uninstalled.
        admin.clear_uninstalled(id);
        assert!(!admin.is_uninstalled(id));
    }

    #[test]
    fn uninstalled_set_is_independent_of_pending_restart() {
        let admin = test_admin();
        let id = "com.acme.purgeme";
        // The two markers are orthogonal: a purge clears pending-restart and
        // sets uninstalled, and neither bleeds into the other's predicate.
        admin.mark_uninstalled(id);
        assert!(admin.is_uninstalled(id));
        assert!(!admin.is_pending_restart(id));
    }
}
