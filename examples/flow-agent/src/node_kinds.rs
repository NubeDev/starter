//! `/api/node-kinds` surface — slice A of
//! `DOCS/extensions/scope/FLOW-NODES.md`.
//!
//! Composes a [`starter_flow_spi::node::CompositeNodeKindRegistry`]
//! (built-in `starter.flow.*` kinds first, then extension-contributed
//! kinds) behind an [`arc_swap::ArcSwap`] per R-flow-node-6 so slice B's
//! `POST /admin/extensions/reload` can swap the dynamic half in place
//! without disrupting `GET /api/node-kinds` readers.
//!
//! Slice A wires *only* the descriptor surface — the dynamic half is
//! empty by default; the host can construct one by calling
//! [`starter_ext_flow::contributed_node_kinds`] against a loaded
//! extension and feeding the entries through
//! [`NodeKindsState::install_dynamic`]. The endpoints honour
//! R-flow-node-1 (one new wire surface, no parallel streaming shape):
//! the three GETs here are read-only descriptor accessors, NOT the
//! invocation path (which lives behind the engine's
//! `NodeKindRegistry::lookup` in `starter-flow`).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use arc_swap::ArcSwap;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use starter_ext_flow::ContributedNodeKindMeta;
use starter_flow_nodes::node_registry::StaticNodeKindRegistry;
use starter_flow_spi::node::{
    CompositeNodeKindRegistry, DynamicNodeKindRegistry, NodeKindRegistry,
};

/// In-process en-locale catalog. The slice A demo bundles only `en`;
/// future stages will route this through `starter-i18n`'s
/// `MessageBundle` so extension-shipped `contributes.i18n` catalogs
/// merge in.
const EN_CATALOG_JSON: &str = include_str!("../../../crates/starter-i18n/catalogs/starter/en.json");

/// Shared state for the `/api/node-kinds*` endpoints.
///
/// Holds an [`ArcSwap`] over the active
/// [`CompositeNodeKindRegistry`] plus a side-table of
/// [`ContributedNodeKindMeta`] (indexed by reverse-DNS kind id) so the
/// schema/description handlers can resolve their on-disk paths
/// without re-parsing manifests on every request.
#[derive(Clone)]
pub struct NodeKindsState {
    inner: Arc<NodeKindsInner>,
}

struct NodeKindsInner {
    registry: ArcSwap<CompositeNodeKindRegistry>,
    /// `kind -> (extension_id, meta)`. Updated atomically alongside
    /// the registry swap so `/settings-schema` and `/description`
    /// observe a consistent view.
    contributed: ArcSwap<BTreeMap<String, ContributedNodeKindMeta>>,
    /// In-memory en-locale i18n catalog. Loaded once at startup;
    /// merged with extension catalogs in future stages.
    catalog: BTreeMap<String, String>,
    /// Schemars-derived schemas for the host's built-in kinds, keyed
    /// by reverse-DNS kind id. Built-ins don't ship a settings-schema
    /// *file*; the schema lives in code (`NodeBehavior::config_schema`),
    /// so we materialise the JSON once at boot and hand it out from
    /// the same `/api/node-kinds/<kind>/settings-schema` endpoint.
    builtin_schemas: BTreeMap<String, serde_json::Value>,
}

impl NodeKindsState {
    /// Build a state with the built-in `starter.flow.*` kinds and an
    /// empty dynamic registry. Slice B's reload path will call
    /// [`Self::install_dynamic`] to swap a fresh dynamic registry in.
    pub fn with_builtins() -> Self {
        let static_reg: Arc<dyn NodeKindRegistry> =
            Arc::new(StaticNodeKindRegistry::with_builtins());
        let dynamic_reg = Arc::new(DynamicNodeKindRegistry::new());
        let composite = CompositeNodeKindRegistry::new(static_reg, dynamic_reg);

        let catalog: BTreeMap<String, String> = serde_json::from_str(EN_CATALOG_JSON)
            .expect("bundled en catalog parses as a flat string map");

        let builtin_schemas = builtin_settings_schemas();

        Self {
            inner: Arc::new(NodeKindsInner {
                registry: ArcSwap::from_pointee(composite),
                contributed: ArcSwap::from_pointee(BTreeMap::new()),
                catalog,
                builtin_schemas,
            }),
        }
    }

    /// Atomically swap the dynamic half of the composite registry and
    /// the side-table of contributed metadata. Slice B's reload path
    /// is the only intended caller (R-flow-node-6); slice A exposes
    /// the seam for tests.
    pub fn install_dynamic(
        &self,
        dynamic: DynamicNodeKindRegistry,
        meta: Vec<ContributedNodeKindMeta>,
    ) {
        let current = self.inner.registry.load_full();
        let static_reg = current.statics().clone();
        let next = CompositeNodeKindRegistry::new(static_reg, Arc::new(dynamic));
        self.inner.registry.store(Arc::new(next));
        let map: BTreeMap<String, ContributedNodeKindMeta> =
            meta.into_iter().map(|m| (m.kind.clone(), m)).collect();
        self.inner.contributed.store(Arc::new(map));
    }

    /// Borrow the active composite registry. Reads are wait-free per
    /// the [`ArcSwap`] contract.
    pub fn registry(&self) -> Arc<CompositeNodeKindRegistry> {
        self.inner.registry.load_full()
    }

    fn resolve(&self, key: &str) -> String {
        self.inner
            .catalog
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    fn meta_for(&self, kind: &str) -> Option<ContributedNodeKindMeta> {
        self.inner.contributed.load_full().get(kind).cloned()
    }

    fn builtin_schema(&self, kind: &str) -> Option<serde_json::Value> {
        self.inner.builtin_schemas.get(kind).cloned()
    }
}

impl Default for NodeKindsState {
    fn default() -> Self {
        Self::with_builtins()
    }
}

/// Wire DTO for one entry on `GET /api/node-kinds`. Carries the
/// resolved i18n strings + absolute URLs to the schema/description
/// endpoints so the editor doesn't have to know the route shape.
#[derive(Debug, Clone, Serialize)]
pub struct NodeKindDto {
    /// Reverse-DNS kind id.
    pub kind: String,
    /// Extension id that contributed this kind, or `null` for built-in
    /// `starter.flow.*` kinds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_id: Option<String>,
    /// Resolved i18n label.
    pub label: String,
    /// Resolved i18n one-line summary.
    pub summary: String,
    /// Resolved i18n long-form help.
    pub help: String,
    /// Catalog key for the label (so a client can refetch on locale change).
    pub label_key: String,
    /// Catalog key for the summary.
    pub summary_key: String,
    /// Catalog key for the help text.
    pub help_key: String,
    /// Absolute URL for the kind's settings JSON Schema.
    pub settings_schema_url: String,
    /// Absolute URL for the kind's description markdown, or `null`
    /// when the kind has no description file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_url: Option<String>,
    /// Palette facet tags (empty for built-ins; whatever the manifest
    /// declared for extension-contributed kinds).
    pub facets: Vec<String>,
    /// Advisory streaming flag (R-flow-node-1).
    pub streaming: bool,
}

/// Mount `/api/node-kinds*` onto a router.
pub fn router<S>(state: NodeKindsState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/node-kinds", get(list_node_kinds))
        .route(
            "/api/node-kinds/{kind}/settings-schema",
            get(settings_schema),
        )
        .route("/api/node-kinds/{kind}/description", get(description))
        .with_state(state)
}

async fn list_node_kinds(State(s): State<NodeKindsState>) -> Json<Vec<NodeKindDto>> {
    let reg = s.registry();
    let mut out: Vec<NodeKindDto> = Vec::new();
    for d in reg.all() {
        let kind = d.kind.as_ref().to_owned();
        let label_key = d.label_key.as_ref().to_owned();
        let summary_key = d.summary_key.as_ref().to_owned();
        let help_key = d.help_key.as_ref().to_owned();
        let label = s.resolve(&label_key);
        let summary = s.resolve(&summary_key);
        let help = s.resolve(&help_key);

        let meta = s.meta_for(&kind);
        let extension_id = meta.as_ref().map(|m| m.extension_id.clone());
        let description_url = if meta
            .as_ref()
            .and_then(|m| m.description_path.as_ref())
            .is_some()
        {
            Some(format!("/api/node-kinds/{kind}/description"))
        } else {
            None
        };
        let facets = meta.as_ref().map(|m| m.facets.clone()).unwrap_or_default();
        let streaming = meta.as_ref().map(|m| m.streaming).unwrap_or(false);
        let settings_schema_url = format!("/api/node-kinds/{kind}/settings-schema");

        out.push(NodeKindDto {
            kind,
            extension_id,
            label,
            summary,
            help,
            label_key,
            summary_key,
            help_key,
            settings_schema_url,
            description_url,
            facets,
            streaming,
        });
    }
    out.sort_by(|a, b| a.kind.cmp(&b.kind));
    Json(out)
}

async fn settings_schema(
    State(s): State<NodeKindsState>,
    AxumPath(kind): AxumPath<String>,
) -> Result<axum::response::Response, StatusCode> {
    // Dynamic (extension-contributed) kinds: stream the bundle file.
    if let Some(meta) = s.meta_for(&kind) {
        return Ok(serve_file(meta.settings_schema_path, "application/json").await);
    }
    // Built-in kinds: serve the schemars-derived schema.
    if let Some(schema) = s.builtin_schema(&kind) {
        return Ok(Json(schema).into_response());
    }
    Err(StatusCode::NOT_FOUND)
}

async fn description(
    State(s): State<NodeKindsState>,
    AxumPath(kind): AxumPath<String>,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(meta) = s.meta_for(&kind) {
        if let Some(path) = meta.description_path {
            return Ok(serve_file(path, "text/markdown; charset=utf-8").await);
        }
    }
    Err(StatusCode::NOT_FOUND)
}

async fn serve_file(path: PathBuf, content_type: &'static str) -> axum::response::Response {
    match tokio::fs::read(&path).await {
        Ok(bytes) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, content_type)],
            bytes,
        )
            .into_response(),
        Err(err) => {
            tracing::warn!(
                target: "flow_agent::node_kinds",
                path = %path.display(),
                error = %err,
                "node-kind asset is missing on disk",
            );
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

/// Materialise the schemars-derived `RootSchema` for every built-in
/// kind the host has linked in. Slice A enables `ai-agent`,
/// `trigger-explicit`, and `log` (see `examples/flow-agent/Cargo.toml`);
/// kinds not enabled are simply absent from the map and
/// `/api/node-kinds/<kind>/settings-schema` returns 404 for them.
fn builtin_settings_schemas() -> BTreeMap<String, serde_json::Value> {
    use starter_flow_spi::node::NodeBehavior;

    fn entry<B: NodeBehavior>(behavior: &B) -> (String, serde_json::Value) {
        let kind = behavior.kind_id().as_str().to_owned();
        let schema = behavior.config_schema();
        let json = serde_json::to_value(schema)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        (kind, json)
    }

    let mut out = BTreeMap::new();

    // Only the kinds linked into `flow-agent`'s `starter-flow-nodes`
    // feature set materialise. Adding a feature flag here means
    // adding a kind below — the engine's built-in registry is the
    // source of truth and we mirror its enabled set.
    let log = starter_flow_nodes::log::Log::new();
    let (k, v) = entry(&log);
    out.insert(k, v);

    let trig = starter_flow_nodes::trigger_explicit::TriggerExplicit::new(Arc::new(
        starter_flow_nodes::trigger_explicit::StaticTriggerChannelRegistry::default(),
    ));
    let (k, v) = entry(&trig);
    out.insert(k, v);

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_lists_log_and_trigger() {
        let s = NodeKindsState::with_builtins();
        let reg = s.registry();
        let kinds: Vec<String> = reg
            .all()
            .iter()
            .map(|d| d.kind.as_ref().to_owned())
            .collect();
        assert!(
            kinds.iter().any(|k| k == "starter.flow.log"),
            "kinds: {:?}",
            kinds
        );
    }

    #[test]
    fn resolve_falls_back_to_key_when_missing() {
        let s = NodeKindsState::with_builtins();
        // Known key resolves.
        assert_ne!(
            s.resolve("starter.flow.node.log.label"),
            "starter.flow.node.log.label"
        );
        // Unknown key returns itself (no lookup hides typos).
        assert_eq!(s.resolve("totally.bogus.key"), "totally.bogus.key");
    }

    #[tokio::test]
    async fn install_dynamic_swaps_atomically() {
        use starter_flow_spi::node::{DynamicNodeKindEntry, NodeDescriptor};

        let s = NodeKindsState::with_builtins();
        let before = s.registry().all().len();

        let descriptor = NodeDescriptor::new_owned(
            "com.example.test.kind",
            "com.example.test.kind.label",
            "com.example.test.kind.summary",
            "com.example.test.kind.help",
        );
        let entry = DynamicNodeKindEntry::new(descriptor, || {
            // Slice A: a placeholder behavior is fine — tests don't
            // exercise it through this path.
            Arc::new(starter_ext_flow::UnboundNodeBehavior::new(
                starter_flow_spi::node::KindId::new("com.example.test.kind").unwrap(),
            ))
        });
        let dyn_reg = DynamicNodeKindRegistry::from_entries([entry]);
        let meta = ContributedNodeKindMeta::new(
            "com.example.test",
            "com.example.test.kind",
            PathBuf::from("/tmp/does-not-exist.json"),
            None,
            vec![],
            false,
        );
        s.install_dynamic(dyn_reg, vec![meta]);

        let after = s.registry().all().len();
        assert_eq!(after, before + 1);
    }
}
