//! Handles the admin router pulls registry data from.
//!
//! Built once at boot and threaded into the admin router. Every
//! field is optional so a developer-mode rubix-agent (no warehouse,
//! no extensions) still serves a working — if mostly empty —
//! `/api/v1/admin/*` surface.

use std::collections::HashMap;
use std::sync::Arc;

use rubix_tools::cleaner::RuleRegistry;
use starter_ext_host::{ExtensionRegistry, TemplateRegistry};
use starter_flow_spi::node::NodeBehavior;
use starter_spi::tool::Tool;

/// Snapshot of every registry handle the admin transport needs.
///
/// All fields are `Arc`-backed so the router can clone the state
/// cheaply per-request. Constructing an [`AdminState`] does no I/O
/// — every projector is a pure walk of the supplied handles.
#[derive(Clone, Default)]
pub struct AdminState {
    /// Tool id → live [`Tool`] handle. The dispatcher already
    /// owns one of these for `POST /api/v1/tools/{id}`; the admin
    /// router consumes the same map so a registered tool appears
    /// in both surfaces with no chance of drift.
    pub tools: Arc<HashMap<String, Arc<dyn Tool>>>,

    /// Built-in flow node behaviours. The runtime
    /// [`starter_flow::registry::NodeKindRegistry`] hides its
    /// content behind async locks; the admin surface needs only
    /// the static metadata each [`NodeBehavior`] exposes, so we
    /// thread the same `Vec` the boot path used to seed the live
    /// registry.
    pub node_behaviors: Arc<Vec<Arc<dyn NodeBehavior>>>,

    /// Cleaner anomaly-rule registry (builtins + every Validated
    /// extension's contributions). `None` when warehouse wiring
    /// is disabled — the cleaner is not built in that mode.
    pub rules: Option<Arc<RuleRegistry>>,

    /// Warehouse-read template registry (builtins + contributions).
    /// `None` when no warehouse client is configured.
    pub templates: Option<Arc<TemplateRegistry>>,

    /// Validated + failed extension records. `None` when the
    /// extension host is disabled in config.
    pub extensions: Option<Arc<ExtensionRegistry>>,

    /// Opt-in cache layer (the one shared by the builtin and
    /// process REST dispatchers). `None` when the cache is not
    /// wired — e.g. dev rigs that disable extensions. The admin
    /// surface uses it to expose per-spec hit/miss numbers so the
    /// canary's "is `warehouse_query` paying off?" question has a
    /// concrete answer.
    pub cache_layer: Option<starter_cache::CacheLayer>,

    /// Registered cache specs (one per loaded `*.cache.yaml`
    /// sidecar). `None` when the cache is not wired. The admin
    /// surface joins this with [`Self::cache_layer`]'s per-spec
    /// counters so an operator can see registered-but-never-hit
    /// specs (the "is this kind ever called?" diagnostic).
    pub cache_registry: Option<starter_ext_server::KindCacheRegistry>,
}

impl AdminState {
    /// Empty state — every projector emits an empty page. Useful
    /// for tests that need the router type but not the data.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builder: set the tool map.
    pub fn with_tools(mut self, tools: Arc<HashMap<String, Arc<dyn Tool>>>) -> Self {
        self.tools = tools;
        self
    }

    /// Builder: set the node-behaviour list.
    pub fn with_node_behaviors(mut self, behaviors: Arc<Vec<Arc<dyn NodeBehavior>>>) -> Self {
        self.node_behaviors = behaviors;
        self
    }

    /// Builder: set the cleaner rule registry.
    pub fn with_rules(mut self, rules: Arc<RuleRegistry>) -> Self {
        self.rules = Some(rules);
        self
    }

    /// Builder: set the warehouse template registry.
    pub fn with_templates(mut self, templates: Arc<TemplateRegistry>) -> Self {
        self.templates = Some(templates);
        self
    }

    /// Builder: set the extension registry.
    pub fn with_extensions(mut self, extensions: Arc<ExtensionRegistry>) -> Self {
        self.extensions = Some(extensions);
        self
    }

    /// Builder: set the opt-in cache layer.
    pub fn with_cache_layer(mut self, cache_layer: starter_cache::CacheLayer) -> Self {
        self.cache_layer = Some(cache_layer);
        self
    }

    /// Builder: set the registered cache spec map.
    pub fn with_cache_registry(mut self, registry: starter_ext_server::KindCacheRegistry) -> Self {
        self.cache_registry = Some(registry);
        self
    }
}
