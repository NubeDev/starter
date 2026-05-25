//! Typed flow-definition body shape.
//!
//! `DOCS/flow/scope/settings.md` S-3 + `DOCS/flow/scope/hot-reload.md`
//! HR1 jointly specify the on-wire / on-disk shape a flow body takes:
//!
//! ```yaml
//! flow_id: examples.notes.codeless-demo
//! apply_policy: drain
//! nodes:
//!   - id: agent
//!     kind: starter.flow.ai-agent
//!     settings:
//!       provider_id: anthropic/claude-sonnet-4-6
//!       cost_cap: 0.25
//! links:
//!   - { from: agent.output, to: logger.value }
//! ```
//!
//! This module is the typed projection — `serde` derives both ways so
//! every adapter (REST handler, CLI, file-watcher, extension) parses
//! and serialises through one struct. The `apply_policy` field is
//! optional and defaults to [`ApplyPolicy::Drain`] per HR4 D-HR3.
//!
//! The body is intentionally permissive about unknown top-level
//! fields (no `#[serde(deny_unknown_fields)]` on [`FlowBody`]): the
//! `FlowRevision::body` blob may carry forward-compatible extensions
//! (e.g. a future `metadata:` block) that this struct doesn't model
//! yet. Per-node `settings:` is the validation surface where
//! `deny_unknown_fields` *does* matter — that check lives on each
//! kind's `Settings` derive per settings.md S-1, enforced through
//! `NodeBehavior::validate_settings`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use starter_flow_spi::definition::ApplyPolicy;
use starter_flow_spi::flow::FlowId;
use starter_flow_spi::node::{KindId, NodeId};

/// Typed projection of a flow-definition JSON / YAML body.
///
/// Round-trips through `serde_json::Value` losslessly for the fields
/// it models; unknown top-level fields are dropped on deserialise
/// (callers that need to preserve them carry the raw `Value`
/// separately, e.g. on the `FlowRevision::body` blob).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FlowBody {
    /// The flow id the body declares. Cross-checked against the
    /// `flow_id` argument to `DefinitionManager::publish`; mismatch
    /// is a publish-time error.
    pub flow_id: FlowId,
    /// How structural edits of this body apply to in-flight runs.
    /// Defaults to [`ApplyPolicy::Drain`] when absent.
    #[serde(default)]
    pub apply_policy: ApplyPolicy,
    /// Declared nodes, in body order. Duplicate ids are a
    /// publish-time error.
    #[serde(default)]
    pub nodes: Vec<NodeDecl>,
    /// Declared slot-to-slot links. Both endpoints must reference a
    /// node declared in [`Self::nodes`]; unknown endpoint nodes are
    /// a publish-time error.
    #[serde(default)]
    pub links: Vec<LinkDecl>,
}

impl FlowBody {
    /// Construct an empty body for `flow_id`.
    pub fn new(flow_id: FlowId) -> Self {
        Self {
            flow_id,
            apply_policy: ApplyPolicy::default(),
            nodes: Vec::new(),
            links: Vec::new(),
        }
    }
}

/// One node declaration inside a [`FlowBody`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NodeDecl {
    /// Node id (reverse-DNS). Must be unique within the body.
    pub id: NodeId,
    /// Node-kind id (reverse-DNS). Must resolve in the engine's
    /// `NodeKindRegistry` at publish time; unknown kinds surface as
    /// a [`crate::definition::TopologyResolverError::UnknownKind`].
    pub kind: KindId,
    /// Per-node config bag, validated against the kind's schema at
    /// publish time via
    /// [`NodeBehavior::validate_settings`](starter_flow_spi::node::NodeBehavior::validate_settings).
    /// Defaults to `{}` when absent.
    #[serde(default = "default_settings")]
    pub settings: serde_json::Value,
    /// Per-node trigger inputs (slot names that, when written, fire
    /// the node's `invoke`). Defaults to an empty set; a node with
    /// no trigger inputs is fired only via direct activation.
    #[serde(default)]
    pub triggers: Vec<String>,
    /// Editor-only spatial hint for the graph canvas. Purely
    /// metadata — the classifier ignores it (see
    /// [`structural_delta`]) so dragging a node never reloads
    /// topology. `None` means "no opinion; the canvas may
    /// auto-layout".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<NodePosition>,
}

/// Canvas-space coordinates for a node, in the same units the
/// frontend `<FlowCanvas>` uses. Float so subpixel snapping survives
/// a round-trip. Persisted purely as metadata; ignored by the
/// engine.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NodePosition {
    /// X coordinate in canvas-space pixels.
    pub x: f64,
    /// Y coordinate in canvas-space pixels.
    pub y: f64,
}

fn default_settings() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

impl NodeDecl {
    /// Minimal constructor used by tests and programmatic builders.
    pub fn new(id: NodeId, kind: KindId) -> Self {
        Self {
            id,
            kind,
            settings: default_settings(),
            triggers: Vec::new(),
            position: None,
        }
    }
}

/// One link declaration inside a [`FlowBody`].
///
/// The wire shape is a pair of `<node_id>.<slot_name>` strings, the
/// same shape `DOCS/flow/scope/settings.md` and `hot-reload.md` use
/// in their YAML examples. The dot-separator parse is the SPI
/// `SlotRef` constructor's responsibility — the parser lives in
/// [`crate::definition::resolver`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LinkDecl {
    /// Source slot, formatted as `<node_id>.<slot_name>`.
    pub from: String,
    /// Destination slot, formatted as `<node_id>.<slot_name>`.
    pub to: String,
}

impl LinkDecl {
    /// Construct a link declaration.
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

/// Convenience: pull a flow body out of the opaque
/// [`FlowRevision::body`](starter_flow_spi::flow::FlowRevision::body)
/// blob.
///
/// Used by [`crate::definition::TopologyResolver::resolve`] and the
/// HR-2 edit classifier. Returns a `serde_json` error on a body that
/// doesn't match the typed shape — surfaced upstream as a
/// [`crate::definition::TopologyResolverError::BodyShape`].
pub fn parse_body(value: &serde_json::Value) -> Result<FlowBody, serde_json::Error> {
    serde_json::from_value(value.clone())
}

/// Convenience: extract the settings deltas between two bodies as a
/// map keyed by `(node_id, field_name)`.
///
/// Used by the HR-2 edit classifier; landed here so the
/// settings-projection shape is owned by the body module rather than
/// scattered across the classifier and the resolver. Phase HR-1
/// exposes the helper for tests of the typed body round-trip; the
/// classifier itself lands HR-2.
pub fn settings_map(body: &FlowBody) -> BTreeMap<NodeId, serde_json::Value> {
    body.nodes
        .iter()
        .map(|n| (n.id.clone(), n.settings.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fid() -> FlowId {
        FlowId::new("examples.notes.demo").unwrap()
    }

    #[test]
    fn roundtrip_minimal_body() {
        let body = FlowBody::new(fid());
        let json = serde_json::to_value(&body).unwrap();
        let back: FlowBody = serde_json::from_value(json).unwrap();
        assert_eq!(body, back);
        assert_eq!(back.apply_policy, ApplyPolicy::Drain);
    }

    #[test]
    fn apply_policy_omitted_defaults_to_drain() {
        let json = serde_json::json!({
            "flow_id": "examples.notes.demo",
            "nodes": [],
            "links": []
        });
        let body: FlowBody = serde_json::from_value(json).unwrap();
        assert_eq!(body.apply_policy, ApplyPolicy::Drain);
    }

    #[test]
    fn unknown_top_level_field_is_tolerated() {
        let json = serde_json::json!({
            "flow_id": "examples.notes.demo",
            "metadata": {"author": "ap"},
            "nodes": [],
            "links": []
        });
        let body: FlowBody = serde_json::from_value(json).expect("unknown field tolerated");
        assert_eq!(body.flow_id, fid());
    }

    #[test]
    fn node_decl_settings_default_to_empty_object() {
        let json = serde_json::json!({
            "id": "test.agent",
            "kind": "starter.flow.ai-agent"
        });
        let node: NodeDecl = serde_json::from_value(json).unwrap();
        assert_eq!(node.settings, serde_json::json!({}));
        assert!(node.triggers.is_empty());
    }
}
