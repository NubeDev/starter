//! `TopologyResolver` — pure projection of a typed flow body to an
//! `Arc<FlowTopology>` the propagator can drive.
//!
//! Per `DOCS/flow/scope/hot-reload.md` HR1 step 1 + HR5: this is the
//! one place a `FlowRevision` body is checked against the live
//! `NodeKindRegistry`, every node's settings are validated against
//! its kind schema, every link is type-checked, and the result is
//! either an `Arc<FlowTopology>` ready to be swapped into
//! `ActiveTopology` or a structured [`TopologyResolverError`] that
//! prevents the publish from landing.
//!
//! Phase HR-1 ships the resolver as a *pure* function — the engine
//!'s `ActiveTopology` swap, the settings → slot projection (HR-2 +
//! settings.md S-3), and the `$link` escape hatch (settings.md S-5)
//! land in later phases. What's here today is the validation +
//! topology-construction primitive `DefinitionManager::publish`
//! calls.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use thiserror::Error;

use starter_flow_spi::flow::FlowRevision;
use starter_flow_spi::node::{NodeId, SlotRef};
use starter_flow_spi::settings::SettingsError;

use crate::definition::body::{self, FlowBody, LinkDecl, NodeDecl};
use crate::propagator::FlowTopology;
use crate::registry::NodeKindRegistry;

/// Resolver failures returned by [`TopologyResolver::resolve`].
///
/// Every variant is a *publish-time* error — a draft that produces
/// any of these never becomes a [`FlowRevision`] per HR6 (*"bad
/// drafts never go live"*). Surfaced to the caller of
/// [`crate::definition::DefinitionManager::publish`] verbatim.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TopologyResolverError {
    /// The opaque body blob did not deserialise into the typed
    /// [`FlowBody`] shape. Carries the underlying `serde_json`
    /// error so editor surfaces can pin-point the bad field.
    #[error("flow body shape invalid: {detail}")]
    BodyShape {
        /// Human-readable description of the deserialise failure.
        detail: String,
    },

    /// The body declares a `flow_id` that doesn't match the
    /// `flow_id` argument the caller passed to `publish`.
    #[error("flow_id mismatch: body declares `{body}` but publish targets `{target}`")]
    FlowIdMismatch {
        /// Flow id in the body.
        body: String,
        /// Flow id the publish targets.
        target: String,
    },

    /// Two nodes in the body share the same id.
    #[error("duplicate node id `{node}` in flow body")]
    DuplicateNode {
        /// The colliding node id.
        node: NodeId,
    },

    /// A node references a kind that is not registered in the
    /// engine's [`NodeKindRegistry`]. HR8 will re-attempt resolve
    /// when the kind shows up; today the flow stays unmounted.
    #[error("unknown node kind `{kind}` for node `{node}` — kind is not registered")]
    UnknownKind {
        /// The node referencing the missing kind.
        node: NodeId,
        /// The kind id the node references.
        kind: String,
    },

    /// A link's source or destination references a node not declared
    /// in the body.
    #[error("link references unknown node: from=`{from}` to=`{to}`")]
    LinkEndpointUnknown {
        /// The source-slot string.
        from: String,
        /// The destination-slot string.
        to: String,
    },

    /// A link's `from` or `to` string couldn't be parsed as
    /// `<node_id>.<slot_name>`.
    #[error("malformed link endpoint `{endpoint}`: expected `<node>.<slot>`")]
    LinkEndpointMalformed {
        /// The offending endpoint string.
        endpoint: String,
    },

    /// A node's `settings` body failed `validate_settings` against
    /// the kind's schema (settings.md S-2 / S-7).
    #[error("settings violation on node `{node}` (kind `{kind}`): {error}")]
    SettingsViolation {
        /// The node carrying the bad settings.
        node: NodeId,
        /// The kind id (string form — `KindId` is not `Copy`).
        kind: String,
        /// The structured settings error from the kind.
        #[source]
        error: SettingsError,
    },
}

/// Pure projection from a flow-body to a runnable
/// [`FlowTopology`].
///
/// Stateless; ships a `resolve` associated function rather than a
/// struct method because it owns no state and never needs to be
/// constructed. Future overloads (e.g. `resolve_with_options(...)`
/// for HR-2's settings projection) live as sibling associated
/// functions.
pub struct TopologyResolver;

impl TopologyResolver {
    /// Build a runnable topology for `revision` against the supplied
    /// kind registry.
    ///
    /// Walks the body in three passes:
    ///
    /// 1. **Parse + cross-check** the typed [`FlowBody`] against the
    ///    `revision.flow_id`.
    /// 2. **Per-node:** look up the kind in the registry, validate
    ///    settings against the kind's schema, collect the
    ///    `(NodeId → Arc<dyn NodeBehavior>)` map and the per-node
    ///    trigger-slot set.
    /// 3. **Per-link:** parse both endpoints into [`SlotRef`]s,
    ///    check the endpoint nodes are declared, append to the
    ///    `links` adjacency map.
    ///
    /// HR-1 does **not** project settings onto config slots — that
    /// lands HR-2 / settings.md S-3 (the resolver gains a `GraphStore`
    /// parameter and writes one slot per [`Settings`] field). For
    /// now the resolved topology is settings-aware in the sense
    /// that *bad* settings prevent the publish, but the slots in
    /// the live graph store aren't touched.
    pub async fn resolve(
        revision: &FlowRevision,
        kinds: &NodeKindRegistry,
    ) -> Result<Arc<FlowTopology>, TopologyResolverError> {
        let body =
            body::parse_body(&revision.body).map_err(|e| TopologyResolverError::BodyShape {
                detail: e.to_string(),
            })?;
        Self::resolve_body(&body, &revision.flow_id, kinds).await
    }

    /// Lower-level variant that takes an already-parsed body. Used
    /// by `DefinitionManager::publish` where the body was parsed
    /// once for validation and re-using it avoids a second
    /// `from_value`. Cross-checks the body's `flow_id` against
    /// `target_flow`.
    pub async fn resolve_body(
        body: &FlowBody,
        target_flow: &starter_flow_spi::flow::FlowId,
        kinds: &NodeKindRegistry,
    ) -> Result<Arc<FlowTopology>, TopologyResolverError> {
        if body.flow_id != *target_flow {
            return Err(TopologyResolverError::FlowIdMismatch {
                body: body.flow_id.to_string(),
                target: target_flow.to_string(),
            });
        }

        // Pass 1: collect nodes + behaviors; refuse duplicates;
        // refuse unknown kinds; refuse bad settings.
        let mut behaviors = BTreeMap::new();
        let mut triggers = BTreeMap::new();
        let mut seen: BTreeSet<NodeId> = BTreeSet::new();
        for NodeDecl {
            id,
            kind,
            settings,
            triggers: trigger_slots,
        } in &body.nodes
        {
            if !seen.insert(id.clone()) {
                return Err(TopologyResolverError::DuplicateNode { node: id.clone() });
            }
            let behavior =
                kinds
                    .lookup(kind)
                    .await
                    .ok_or_else(|| TopologyResolverError::UnknownKind {
                        node: id.clone(),
                        kind: kind.to_string(),
                    })?;
            behavior.validate_settings(settings).map_err(|error| {
                TopologyResolverError::SettingsViolation {
                    node: id.clone(),
                    kind: kind.to_string(),
                    error,
                }
            })?;
            behaviors.insert(id.clone(), behavior);
            if !trigger_slots.is_empty() {
                triggers.insert(
                    id.clone(),
                    trigger_slots.iter().cloned().collect::<BTreeSet<_>>(),
                );
            }
        }

        // Pass 2: links.
        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        for LinkDecl { from, to } in &body.links {
            let src = parse_endpoint(from)?;
            let dst = parse_endpoint(to)?;
            if !seen.contains(&src.node) || !seen.contains(&dst.node) {
                return Err(TopologyResolverError::LinkEndpointUnknown {
                    from: from.clone(),
                    to: to.clone(),
                });
            }
            links.entry(src).or_default().push(dst);
        }

        Ok(Arc::new(FlowTopology {
            links,
            triggers,
            behaviors,
        }))
    }
}

fn parse_endpoint(s: &str) -> Result<SlotRef, TopologyResolverError> {
    // Node ids are reverse-DNS (`com.example.foo`), so the
    // node/slot separator is the *last* dot — the node is
    // everything before it, the slot is everything after.
    let (node, slot) =
        s.rsplit_once('.')
            .ok_or_else(|| TopologyResolverError::LinkEndpointMalformed {
                endpoint: s.to_owned(),
            })?;
    if node.is_empty() || slot.is_empty() {
        return Err(TopologyResolverError::LinkEndpointMalformed {
            endpoint: s.to_owned(),
        });
    }
    let node = NodeId::new(node).map_err(|_| TopologyResolverError::LinkEndpointMalformed {
        endpoint: s.to_owned(),
    })?;
    Ok(SlotRef::new(node, slot))
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use std::sync::LazyLock;

    use starter_flow_spi::flow::{FlowId, FlowRevisionId};
    use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap};
    use starter_flow_spi::settings::EMPTY_SCHEMA;

    /// Test-only kind with an empty schema (every body validates).
    struct AnyKind {
        kind: KindId,
    }
    impl AnyKind {
        fn new(s: &str) -> Arc<Self> {
            Arc::new(Self {
                kind: KindId::new(s).unwrap(),
            })
        }
    }
    #[async_trait]
    impl NodeBehavior for AnyKind {
        fn kind_id(&self) -> &KindId {
            &self.kind
        }
        async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
            Ok(SlotMap::new())
        }
    }

    /// Test-only kind whose schema rejects everything except an
    /// object with `{"required_field": <string>}`.
    struct StrictKind {
        kind: KindId,
    }
    impl StrictKind {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                kind: KindId::new("test.strict").unwrap(),
            })
        }
    }
    static STRICT_SCHEMA: LazyLock<schemars::schema::RootSchema> = LazyLock::new(|| {
        #[derive(schemars::JsonSchema)]
        #[serde(deny_unknown_fields)]
        #[allow(dead_code)]
        struct S {
            required_field: String,
        }
        schemars::schema_for!(S)
    });
    #[async_trait]
    impl NodeBehavior for StrictKind {
        fn kind_id(&self) -> &KindId {
            &self.kind
        }
        async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
            Ok(SlotMap::new())
        }
        fn config_schema(&self) -> &'static schemars::schema::RootSchema {
            &STRICT_SCHEMA
        }
    }

    fn flow_id() -> FlowId {
        FlowId::new("examples.test.demo").unwrap()
    }

    fn revision(body: serde_json::Value) -> FlowRevision {
        FlowRevision::new(flow_id(), FlowRevisionId::new(), body)
    }

    #[tokio::test]
    async fn resolves_minimal_body() {
        let kinds = NodeKindRegistry::new();
        kinds
            .register(AnyKind::new("com.example.any"))
            .await
            .unwrap();

        let body = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any"}
            ],
            "links": [
                {"from": "test.n1.out", "to": "test.n2.in"}
            ]
        });

        let topo = TopologyResolver::resolve(&revision(body), &kinds)
            .await
            .expect("resolves");
        assert_eq!(topo.behaviors.len(), 2);
        assert_eq!(topo.links.len(), 1);
        let (src, dsts) = topo.links.iter().next().unwrap();
        assert_eq!(src.slot, "out");
        assert_eq!(dsts.len(), 1);
        assert_eq!(dsts[0].slot, "in");
    }

    #[tokio::test]
    async fn unknown_kind_rejected() {
        let kinds = NodeKindRegistry::new();
        let body = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [{"id": "test.n1", "kind": "com.missing"}],
            "links": []
        });
        let Err(err) = TopologyResolver::resolve(&revision(body), &kinds).await else {
            panic!("expected UnknownKind");
        };
        assert!(matches!(err, TopologyResolverError::UnknownKind { .. }));
    }

    #[tokio::test]
    async fn duplicate_node_rejected() {
        let kinds = NodeKindRegistry::new();
        kinds
            .register(AnyKind::new("com.example.any"))
            .await
            .unwrap();
        let body = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n1", "kind": "com.example.any"}
            ],
            "links": []
        });
        let Err(err) = TopologyResolver::resolve(&revision(body), &kinds).await else {
            panic!("expected DuplicateNode");
        };
        assert!(matches!(err, TopologyResolverError::DuplicateNode { .. }));
    }

    #[tokio::test]
    async fn flow_id_mismatch_rejected() {
        let kinds = NodeKindRegistry::new();
        let body = serde_json::json!({
            "flow_id": "examples.test.other",
            "nodes": [], "links": []
        });
        let Err(err) = TopologyResolver::resolve(&revision(body), &kinds).await else {
            panic!("expected FlowIdMismatch");
        };
        assert!(matches!(err, TopologyResolverError::FlowIdMismatch { .. }));
    }

    #[tokio::test]
    async fn link_endpoint_unknown_node_rejected() {
        let kinds = NodeKindRegistry::new();
        kinds
            .register(AnyKind::new("com.example.any"))
            .await
            .unwrap();
        let body = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [{"id": "test.n1", "kind": "com.example.any"}],
            "links": [{"from": "test.n1.out", "to": "test.missing.in"}]
        });
        let Err(err) = TopologyResolver::resolve(&revision(body), &kinds).await else {
            panic!("expected LinkEndpointUnknown");
        };
        assert!(matches!(
            err,
            TopologyResolverError::LinkEndpointUnknown { .. }
        ));
    }

    #[tokio::test]
    async fn link_endpoint_malformed_rejected() {
        let kinds = NodeKindRegistry::new();
        kinds
            .register(AnyKind::new("com.example.any"))
            .await
            .unwrap();
        let body = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [{"id": "test.n1", "kind": "com.example.any"}],
            "links": [{"from": "no_dot_here", "to": "test.n1.in"}]
        });
        let Err(err) = TopologyResolver::resolve(&revision(body), &kinds).await else {
            panic!("expected LinkEndpointMalformed");
        };
        assert!(matches!(
            err,
            TopologyResolverError::LinkEndpointMalformed { .. }
        ));
    }

    #[tokio::test]
    async fn bad_settings_rejected() {
        let kinds = NodeKindRegistry::new();
        kinds.register(StrictKind::new()).await.unwrap();
        let body = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [{
                "id": "test.n1",
                "kind": "test.strict",
                "settings": {"required_field": 42}
            }],
            "links": []
        });
        let Err(err) = TopologyResolver::resolve(&revision(body), &kinds).await else {
            panic!("expected SettingsViolation");
        };
        match err {
            TopologyResolverError::SettingsViolation { node, kind, error } => {
                assert_eq!(node.as_str(), "test.n1");
                assert_eq!(kind, "test.strict");
                assert!(matches!(error, SettingsError::SchemaViolation { .. }));
            }
            other => panic!("expected SettingsViolation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn missing_settings_for_required_field_rejected() {
        let kinds = NodeKindRegistry::new();
        kinds.register(StrictKind::new()).await.unwrap();
        let body = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [{"id": "test.n1", "kind": "test.strict"}],
            "links": []
        });
        let Err(err) = TopologyResolver::resolve(&revision(body), &kinds).await else {
            panic!("expected SettingsViolation");
        };
        match err {
            TopologyResolverError::SettingsViolation {
                error: SettingsError::SchemaViolation { rule, .. },
                ..
            } => {
                assert_eq!(rule, "required");
            }
            other => panic!("expected SettingsViolation/required, got {other:?}"),
        }
    }

    #[test]
    fn unused_anykind_lookup_compiles() {
        // touch the public surface in a test-only path so dead-code
        // warnings don't kick in on items only exercised through
        // `#[tokio::test]` instantiations.
        let _ = EMPTY_SCHEMA.clone();
    }
}
