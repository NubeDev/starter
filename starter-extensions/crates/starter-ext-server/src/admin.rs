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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::ExtensionId;
use starter_ext_supervisor::SupervisorHandle;

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
    etag_cache: EtagCache,
    worker_states: Option<WorkerStatesFn>,
    /// On-disk root that holds extension bundles. Required for the
    /// install / uninstall endpoints (Phase D.1); endpoints return
    /// HTTP 503 when unset so a `TestApp` that doesn't wire it stays
    /// functional for the toggle-only surface.
    extensions_dir: Option<PathBuf>,
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

impl ExtensionAdmin {
    /// Start building from a sealed registry.
    pub fn builder(registry: Arc<ExtensionRegistry>) -> ExtensionAdminBuilder {
        ExtensionAdminBuilder {
            registry,
            supervisors: HashMap::new(),
            store: None,
            factory: None,
            worker_states: None,
            extensions_dir: None,
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

    pub(crate) fn store(&self) -> &dyn EnablementStore {
        &*self.inner.store
    }

    pub(crate) fn factory(&self) -> &DynFactory {
        &self.inner.factory
    }

    pub(crate) fn etag_cache(&self) -> &EtagCache {
        &self.inner.etag_cache
    }

    /// Render the worker-state field for `GET /extensions/<id>`.
    /// Returns an empty vector when no provider is wired — the JSON
    /// response still includes a `workers: []` field, which is the
    /// truthful shape for hosts that did not opt into the periodic-
    /// worker adapter.
    /// On-disk root holding extension bundles, when wired. Used by
    /// the install / uninstall endpoints (Phase D.1).
    pub(crate) fn extensions_dir(&self) -> Option<&std::path::Path> {
        self.inner.extensions_dir.as_deref()
    }

    pub(crate) fn worker_states(&self, id: &ExtensionId) -> Vec<serde_json::Value> {
        match &self.inner.worker_states {
            Some(f) => f(id),
            None => Vec::new(),
        }
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
    worker_states: Option<WorkerStatesFn>,
    extensions_dir: Option<PathBuf>,
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

    /// Wire the on-disk extensions root so the install / uninstall
    /// endpoints (Phase D.1) can extract tarballs into it and remove
    /// uninstalled bundles. When unset both endpoints return HTTP 503.
    pub fn with_extensions_dir(mut self, dir: PathBuf) -> Self {
        self.extensions_dir = Some(dir);
        self
    }

    /// Materialise the [`ExtensionAdmin`].
    pub fn build(self) -> ExtensionAdmin {
        ExtensionAdmin {
            inner: Arc::new(Inner {
                registry: self.registry,
                supervisors: RwLock::new(self.supervisors),
                store: self
                    .store
                    .unwrap_or_else(|| Arc::new(InMemoryEnablementStore::new())),
                factory: self
                    .factory
                    .unwrap_or_else(|| Arc::new(DefaultSupervisorFactory)),
                etag_cache: EtagCache::new(),
                worker_states: self.worker_states,
                extensions_dir: self.extensions_dir,
            }),
        }
    }
}
