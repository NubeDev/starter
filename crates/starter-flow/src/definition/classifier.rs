//! Pure diff classifier per `DOCS/flow/scope/hot-reload.md` HR3 +
//! D-HR4.
//!
//! Input: two parsed [`FlowBody`] snapshots (the previous head and
//! the new draft). Output: an [`EditKind`] telling the publish path
//! how to apply the edit.
//!
//! Pure function; no I/O; cheap. Lives next to the resolver because
//! the two share the body-shape knowledge.

use std::collections::BTreeMap;

use starter_flow_spi::definition::EditKindTag;
use starter_flow_spi::node::{NodeId, SlotRef, SlotValue};

use crate::definition::body::{FlowBody, LinkDecl, NodeDecl};

/// What kind of edit a draft represents relative to the previous
/// head. The publish path dispatches on this:
///
/// - [`Self::SettingsOnly`] \u2192 per-delta `GraphStore::write_slot` with
///   `WriteSlotOpts::config()`. No topology swap.
/// - [`Self::Structural`] \u2192 `ActiveTopology::store(new)` only.
/// - [`Self::Mixed`] \u2192 structural swap first, then per-delta
///   settings writes (HR3 order matters: writes must land in the
///   new topology's slot graph).
/// - [`Self::Initial`] \u2192 first publish for the flow; mount only,
///   nothing to compare against.
/// - [`Self::Unchanged`] \u2192 the typed shape is identical between
///   old and new (only forward-compat top-level fields differ);
///   nothing to do.
#[derive(Debug, Clone, PartialEq)]
pub enum EditKind {
    /// First publish for a flow. The HR1 short-circuit cannot fire
    /// (there's no head to compare against) and the swap is an
    /// unconditional atomic mount per HR4's first-publish corner
    /// case.
    Initial,
    /// Bodies match field-for-field on the typed shape; no work for
    /// the engine. The publish itself still writes a new revision
    /// (the body bytes differed, otherwise HR1 short-circuit would
    /// have fired upstream) but no swap and no slot writes are
    /// needed.
    Unchanged,
    /// Only per-node `settings` deltas; no node-set or link-set
    /// change. Carries the resolved slot writes ready for the
    /// settings-path executor.
    SettingsOnly {
        /// Slot writes to perform in order. Each entry is a
        /// `(SlotRef, SlotValue)` derived from a changed settings
        /// field on a node whose `(NodeId, KindId, inbound link set,
        /// outbound link set)` is byte-identical between the old
        /// and new bodies (HR4 live-migrate safety bar).
        writes: Vec<(SlotRef, SlotValue)>,
    },
    /// Links / nodes / kinds / triggers / apply_policy changed.
    /// Settings deltas \u2014 if any \u2014 are folded into the topology
    /// swap (their resolved values are written into the new topology's
    /// slot graph by `TopologyResolver::resolve` once HR-3 wires the
    /// settings projection).
    Structural,
    /// Both structural and settings deltas. The publish path applies
    /// the structural swap first; then runs the settings writes per
    /// the embedded list. Order: structural-then-settings, per HR3.
    Mixed {
        /// Settings writes to apply after the swap. Restricted to
        /// the nodes whose wiring did not shift \u2014 HR4 live-migrate's
        /// safety bar; nodes whose wiring did shift have their
        /// settings already projected into the new topology's slot
        /// graph by the resolver and don't need a separate write.
        writes: Vec<(SlotRef, SlotValue)>,
    },
}

impl EditKind {
    /// Map to the wire-shape tag emitted on
    /// [`FlowDefinitionEvent`](starter_flow_spi::definition::FlowDefinitionEvent)s
    /// and the `flow.definition.publish` tracing span.
    pub fn tag(&self) -> EditKindTag {
        match self {
            Self::Initial => EditKindTag::Initial,
            Self::Unchanged => EditKindTag::Settings, // no-op settings
            Self::SettingsOnly { .. } => EditKindTag::Settings,
            Self::Structural => EditKindTag::Structural,
            Self::Mixed { .. } => EditKindTag::Mixed,
        }
    }
}

/// Diff `new` against `old`.
///
/// Implementation notes:
///
/// - Structural deltas: node-set (by id + kind), per-node trigger
///   set, link-set, and the `apply_policy` field (changing
///   `apply_policy` is itself structural because it changes the
///   contract under which in-flight runs are torn down).
/// - Settings deltas: per-node, the top-level keys of the
///   `settings:` JSON object are projected onto config slots by
///   convention (key name = slot name, per
///   `DOCS/flow/scope/settings.md` Q-S1 (c)).
/// - HR4 live-migrate safety bar: settings writes are emitted only
///   for nodes whose **wiring did not shift** \u2014 same inbound and
///   outbound link sets between old and new. Nodes whose wiring did
///   shift have their settings folded into the topology swap and the
///   classifier does NOT emit a settings write for them (the
///   resolver's projection at swap time embeds the literal value
///   into the new topology's slot graph).
pub fn classify(old: &FlowBody, new: &FlowBody) -> EditKind {
    let structural = structural_delta(old, new);
    let settings_changed = settings_changed(old, new);
    let settings_writes = settings_writes(old, new, structural);

    match (structural, settings_changed) {
        (false, false) => EditKind::Unchanged,
        (false, true) => EditKind::SettingsOnly {
            writes: settings_writes,
        },
        (true, false) => EditKind::Structural,
        (true, true) => EditKind::Mixed {
            writes: settings_writes,
        },
    }
}

/// Whether ANY per-node settings field changed between bodies,
/// regardless of whether the change is safe to project as a live
/// slot write (wiring-shifted nodes still report `true` here).
fn settings_changed(old: &FlowBody, new: &FlowBody) -> bool {
    let old_by_id: BTreeMap<&NodeId, &serde_json::Value> =
        old.nodes.iter().map(|n| (&n.id, &n.settings)).collect();
    for n in &new.nodes {
        let Some(prev_settings) = old_by_id.get(&n.id) else {
            // New node — its settings come along with the topology
            // swap, not a settings delta in the classifier sense.
            continue;
        };
        if *prev_settings != &n.settings {
            return true;
        }
    }
    false
}

/// Whether the structural shape of the body changed.
///
/// Exposed pub(crate) so the publish path can decide whether to
/// re-mount the active topology even when no settings deltas exist
/// (e.g. a link addition with no settings changes).
fn structural_delta(old: &FlowBody, new: &FlowBody) -> bool {
    if old.apply_policy != new.apply_policy {
        return true;
    }
    // Node-set delta: by (id, kind, trigger-set).
    let old_nodes = node_shape_map(old);
    let new_nodes = node_shape_map(new);
    if old_nodes != new_nodes {
        return true;
    }
    // Link-set delta: as an unordered set, since link order in the
    // body is editorial, not semantic. (A future authoring tool that
    // wants deterministic ordering can sort the body before
    // publishing.)
    let mut old_links: Vec<(&str, &str)> =
        old.links.iter().map(|l| (l.from.as_str(), l.to.as_str())).collect();
    let mut new_links: Vec<(&str, &str)> =
        new.links.iter().map(|l| (l.from.as_str(), l.to.as_str())).collect();
    old_links.sort_unstable();
    new_links.sort_unstable();
    old_links != new_links
}

/// Build the per-node settings writes for a body diff.
///
/// `structural` is the result of [`structural_delta`]; when `true`,
/// settings writes are restricted to nodes whose wiring did not
/// shift (HR4 live-migrate safety bar). When `false`, every node's
/// settings delta is a candidate write.
fn settings_writes(
    old: &FlowBody,
    new: &FlowBody,
    structural: bool,
) -> Vec<(SlotRef, SlotValue)> {
    let old_by_id: BTreeMap<&NodeId, &NodeDecl> =
        old.nodes.iter().map(|n| (&n.id, n)).collect();
    let new_by_id: BTreeMap<&NodeId, &NodeDecl> =
        new.nodes.iter().map(|n| (&n.id, n)).collect();

    let mut out = Vec::new();
    for (id, new_node) in &new_by_id {
        let Some(old_node) = old_by_id.get(id) else {
            // New node \u2014 its settings ride the topology swap, not a
            // separate write. The resolver projects them into the
            // new topology's slot graph.
            continue;
        };

        // HR4 live-migrate safety bar: only emit settings writes
        // for nodes whose wiring did not shift. Wiring stability =
        // same kind + same inbound + same outbound link set.
        if structural && wiring_shifted(id, old, new) {
            continue;
        }

        // Per-field diff on the settings object's top-level keys.
        // Convention Q-S1 (c): field name = slot name.
        let old_settings = settings_as_map(&old_node.settings);
        let new_settings = settings_as_map(&new_node.settings);

        let mut keys: std::collections::BTreeSet<&String> = std::collections::BTreeSet::new();
        keys.extend(old_settings.keys().copied());
        keys.extend(new_settings.keys().copied());

        for key in keys {
            let old_v = old_settings.get(key);
            let new_v = new_settings.get(key);
            if old_v == new_v {
                continue;
            }
            // Either added, changed, or removed. A removed field is
            // currently elided (no way to express "delete" via
            // write_slot today; the R2 idempotent short-circuit
            // means a re-introduction with the original value would
            // be a no-op once added). Track as a follow-up; in
            // practice a deny_unknown_fields settings schema means
            // removal is rare \u2014 a kind that drops a field bumps
            // its KindId per S6.
            let Some(value) = new_v else { continue };
            let slot_value = json_to_slot_value((*value).clone());
            out.push((SlotRef::new((*id).clone(), key.clone()), slot_value));
        }
    }
    out
}

fn settings_as_map(v: &serde_json::Value) -> BTreeMap<&String, &serde_json::Value> {
    match v {
        serde_json::Value::Object(map) => map.iter().collect(),
        _ => BTreeMap::new(),
    }
}

fn wiring_shifted(id: &NodeId, old: &FlowBody, new: &FlowBody) -> bool {
    // Same KindId on both sides?
    let old_kind = old.nodes.iter().find(|n| &n.id == id).map(|n| &n.kind);
    let new_kind = new.nodes.iter().find(|n| &n.id == id).map(|n| &n.kind);
    if old_kind != new_kind {
        return true;
    }
    // Same inbound / outbound link set?
    let old_in = inbound(id, &old.links);
    let new_in = inbound(id, &new.links);
    if old_in != new_in {
        return true;
    }
    let old_out = outbound(id, &old.links);
    let new_out = outbound(id, &new.links);
    if old_out != new_out {
        return true;
    }
    // Same trigger-input set?
    let old_trig = old
        .nodes
        .iter()
        .find(|n| &n.id == id)
        .map(|n| {
            n.triggers
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    let new_trig = new
        .nodes
        .iter()
        .find(|n| &n.id == id)
        .map(|n| {
            n.triggers
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    old_trig != new_trig
}

fn inbound<'a>(
    id: &NodeId,
    links: &'a [LinkDecl],
) -> std::collections::BTreeSet<(&'a str, &'a str)> {
    let prefix = id.as_str();
    links
        .iter()
        .filter(|l| {
            l.to.rsplit_once('.')
                .is_some_and(|(node, _)| node == prefix)
        })
        .map(|l| (l.from.as_str(), l.to.as_str()))
        .collect()
}

fn outbound<'a>(
    id: &NodeId,
    links: &'a [LinkDecl],
) -> std::collections::BTreeSet<(&'a str, &'a str)> {
    let prefix = id.as_str();
    links
        .iter()
        .filter(|l| {
            l.from
                .rsplit_once('.')
                .is_some_and(|(node, _)| node == prefix)
        })
        .map(|l| (l.from.as_str(), l.to.as_str()))
        .collect()
}

fn node_shape_map(body: &FlowBody) -> BTreeMap<&NodeId, (&str, Vec<&str>)> {
    body.nodes
        .iter()
        .map(|n| {
            let mut trig: Vec<&str> = n.triggers.iter().map(String::as_str).collect();
            trig.sort_unstable();
            trig.dedup();
            (&n.id, (n.kind.as_str(), trig))
        })
        .collect()
}

/// Convert a `serde_json::Value` to a [`SlotValue`] using the
/// convention every settings projection follows.
///
/// Primitive types fall through to their typed variants; arrays and
/// objects are wrapped in [`SlotValue::Json`]. Integer-valued
/// numbers go to [`SlotValue::Int`] when they fit in `i64`; the
/// fall-back path is [`SlotValue::Float`] which matches the
/// `serde_json::Number` widening for non-integer values.
pub fn json_to_slot_value(value: serde_json::Value) -> SlotValue {
    match value {
        serde_json::Value::Null => SlotValue::Null,
        serde_json::Value::Bool(b) => SlotValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SlotValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                SlotValue::Float(f)
            } else {
                // u64 outside i64 range; carry as JSON rather than
                // truncate.
                SlotValue::Json(serde_json::Value::Number(n))
            }
        }
        serde_json::Value::String(s) => SlotValue::String(s),
        v @ (serde_json::Value::Array(_) | serde_json::Value::Object(_)) => SlotValue::Json(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use starter_flow_spi::definition::ApplyPolicy;
    use starter_flow_spi::flow::FlowId;
    use starter_flow_spi::node::KindId;

    fn fid() -> FlowId {
        FlowId::new("examples.test.demo").unwrap()
    }
    fn nid(s: &str) -> NodeId {
        NodeId::new(s).unwrap()
    }
    fn kid(s: &str) -> KindId {
        KindId::new(s).unwrap()
    }

    fn body_v1() -> FlowBody {
        let mut a = NodeDecl::new(nid("test.agent"), kid("com.example.any"));
        a.settings = serde_json::json!({"prompt": "old", "cost_cap": 0.1});
        let l = NodeDecl::new(nid("test.logger"), kid("com.example.any"));
        FlowBody {
            flow_id: fid(),
            apply_policy: ApplyPolicy::Drain,
            nodes: vec![a, l],
            links: vec![LinkDecl::new("test.agent.output", "test.logger.value")],
        }
    }

    #[test]
    fn settings_only_diff_emits_writes() {
        let mut new = body_v1();
        new.nodes[0].settings = serde_json::json!({"prompt": "new", "cost_cap": 0.1});

        let kind = classify(&body_v1(), &new);
        let EditKind::SettingsOnly { writes } = kind else {
            panic!("expected SettingsOnly");
        };
        assert_eq!(writes.len(), 1);
        let (slot, value) = &writes[0];
        assert_eq!(slot.node.as_str(), "test.agent");
        assert_eq!(slot.slot, "prompt");
        assert!(matches!(value, SlotValue::String(s) if s == "new"));
    }

    #[test]
    fn structural_only_diff_no_writes() {
        let mut new = body_v1();
        new.links
            .push(LinkDecl::new("test.agent.output", "test.logger.alt"));
        let kind = classify(&body_v1(), &new);
        assert_eq!(kind, EditKind::Structural);
    }

    #[test]
    fn mixed_diff_emits_writes_only_for_wiring_stable_nodes() {
        // Change logger's settings (wiring stable) AND add a new link
        // off the agent (wiring shifts for agent).
        let mut new = body_v1();
        new.nodes[1].settings = serde_json::json!({"level": "info"});
        new.links
            .push(LinkDecl::new("test.agent.output", "test.logger.alt"));

        let kind = classify(&body_v1(), &new);
        let EditKind::Mixed { writes } = kind else {
            panic!("expected Mixed, got {kind:?}");
        };
        // Only the logger's settings change is emitted: agent's
        // outbound link set shifted, so its wiring is unstable
        // and its (here unchanged) settings would not have been
        // emitted regardless. The new logger.alt destination is
        // an inbound for logger \u2014 so logger's wiring also
        // shifted! Therefore no writes should be emitted.
        assert!(writes.is_empty(), "wiring shifted for both nodes: {writes:?}");
    }

    #[test]
    fn mixed_diff_emits_settings_write_when_wiring_stable() {
        // Add a brand-new third node + link to it. agent's outbound
        // link set shifts; logger's wiring is unchanged (no new
        // inbound to logger, no removal). Logger's settings change
        // should be emitted.
        let mut new = body_v1();
        let nl = NodeDecl::new(nid("test.notifier"), kid("com.example.any"));
        new.nodes.push(nl);
        new.links
            .push(LinkDecl::new("test.agent.output", "test.notifier.value"));
        new.nodes[1].settings = serde_json::json!({"level": "info"});

        let kind = classify(&body_v1(), &new);
        let EditKind::Mixed { writes } = kind else {
            panic!("expected Mixed, got {kind:?}");
        };
        assert_eq!(writes.len(), 1, "logger's wiring stable: {writes:?}");
        assert_eq!(writes[0].0.node.as_str(), "test.logger");
        assert_eq!(writes[0].0.slot, "level");
    }

    #[test]
    fn apply_policy_change_is_structural() {
        let mut new = body_v1();
        new.apply_policy = ApplyPolicy::Restart;
        let kind = classify(&body_v1(), &new);
        assert_eq!(kind, EditKind::Structural);
    }

    #[test]
    fn unchanged_body_is_unchanged() {
        let kind = classify(&body_v1(), &body_v1());
        assert_eq!(kind, EditKind::Unchanged);
    }

    #[test]
    #[allow(clippy::approx_constant)]
    // 3.14 is a generic float fixture for the f64 branch — not π.
    fn json_conversion_maps_primitives() {
        assert!(matches!(json_to_slot_value(serde_json::json!(null)), SlotValue::Null));
        assert!(matches!(json_to_slot_value(serde_json::json!(true)), SlotValue::Bool(true)));
        assert!(matches!(json_to_slot_value(serde_json::json!(42)), SlotValue::Int(42)));
        assert!(matches!(json_to_slot_value(serde_json::json!(3.14)), SlotValue::Float(_)));
        assert!(matches!(json_to_slot_value(serde_json::json!("hi")), SlotValue::String(_)));
        assert!(matches!(json_to_slot_value(serde_json::json!([1, 2])), SlotValue::Json(_)));
        assert!(matches!(json_to_slot_value(serde_json::json!({"k": "v"})), SlotValue::Json(_)));
    }
}
