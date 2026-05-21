//! Transport-friendly DTOs derived from [`FlowEvent`].
//!
//! Every host that fronts the engine (REST/SSE, gRPC, MCP, JSON-RPC,
//! CLI, Tauri) needs to turn engine `FlowEvent`s into JSON-shaped
//! payloads its transport understands. The conversions are
//! straightforward but easy to drift between hosts — this module
//! centralises the per-event JSON projections so every host renders
//! the same shape on the wire.
//!
//! Lives in `starter-flow-spi` (zero engine deps) so consumers that
//! only need to *interpret* events — without depending on the engine
//! runtime — can do so. Hosts that need to also *subscribe* to events
//! pair this with `RunHandle::events_tx.subscribe()` from
//! `starter-flow` directly.

use serde::{Deserialize, Serialize};

use crate::flow::{FlowEvent, RunId};
use crate::node::{NodeId, SlotValue};

/// JSON projection of [`FlowEvent::NodeEmitted`] suitable for direct
/// inclusion in a transport payload.
///
/// Field names are stable across hosts so a JSON consumer that knows
/// one host knows them all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeSlotValue {
    /// The run that emitted.
    pub run: RunId,
    /// The engine node that emitted. Hosts that surface a separate
    /// UI-node id translate this on the boundary; the engine id is
    /// kept here as the canonical wire value.
    pub node: NodeId,
    /// The output slot name on the emitting node.
    pub slot: String,
    /// The value, JSON-projected via [`slot_value_to_json`].
    pub value: serde_json::Value,
}

impl NodeSlotValue {
    /// Extract a [`NodeSlotValue`] from a [`FlowEvent`], returning
    /// `None` for variants that do not carry a slot emission.
    ///
    /// Hosts typically use this in their event pump:
    ///
    /// ```ignore
    /// while let Ok(ev) = rx.recv().await {
    ///     if let Some(nsv) = NodeSlotValue::from_event(&ev) {
    ///         // forward to SSE / gRPC / MCP
    ///     }
    /// }
    /// ```
    pub fn from_event(ev: &FlowEvent) -> Option<Self> {
        match ev {
            FlowEvent::NodeEmitted {
                run,
                node,
                slot,
                value,
            } => Some(Self {
                run: *run,
                node: node.clone(),
                slot: slot.clone(),
                value: slot_value_to_json(value),
            }),
            _ => None,
        }
    }
}

/// Lossless JSON projection of a [`SlotValue`].
///
/// Behaviour per variant:
/// - `Null` → `null`
/// - `Bool`/`Int`/`Float`/`String` → matching JSON scalar
/// - `Float` that is not finite → `null` (JSON cannot represent NaN/±∞)
/// - `Bytes` → lowercase hex string with `0x` prefix
/// - `Json` → passed through unchanged
///
/// Unknown future variants (the enum is `#[non_exhaustive]`) project
/// to `null` so this helper keeps compiling and producing a sensible
/// JSON value as new slot kinds land.
///
/// The hex projection for `Bytes` is a deliberate trade-off: it is
/// inspector-friendly and works in JSON consumers that don't have a
/// dedicated binary type. Hosts that need a different encoding can
/// build their own projection from the same matcher.
// `non_exhaustive` is for downstream crates; in-crate every known
// variant is matched, so the wildcard arm is currently unreachable.
// Keep it as the forward-compatibility seam.
#[allow(unreachable_patterns)]
pub fn slot_value_to_json(v: &SlotValue) -> serde_json::Value {
    match v {
        SlotValue::Null => serde_json::Value::Null,
        SlotValue::Bool(b) => serde_json::Value::Bool(*b),
        SlotValue::Int(i) => serde_json::Value::Number((*i).into()),
        SlotValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        SlotValue::String(s) => serde_json::Value::String(s.clone()),
        SlotValue::Bytes(b) => {
            let mut s = String::with_capacity(2 + b.len() * 2);
            s.push_str("0x");
            for byte in b {
                use std::fmt::Write as _;
                let _ = write!(s, "{byte:02x}");
            }
            serde_json::Value::String(s)
        }
        SlotValue::Json(j) => j.clone(),
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::{FlowId, RunId};
    use crate::node::NodeId;

    #[test]
    fn slot_value_to_json_covers_every_known_variant() {
        assert_eq!(slot_value_to_json(&SlotValue::Null), serde_json::Value::Null);
        assert_eq!(
            slot_value_to_json(&SlotValue::Bool(true)),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            slot_value_to_json(&SlotValue::Int(-7)),
            serde_json::json!(-7)
        );
        assert_eq!(
            slot_value_to_json(&SlotValue::Float(1.5)),
            serde_json::json!(1.5)
        );
        assert_eq!(
            slot_value_to_json(&SlotValue::Float(f64::NAN)),
            serde_json::Value::Null,
            "non-finite floats project to null since JSON cannot represent them"
        );
        assert_eq!(
            slot_value_to_json(&SlotValue::String("pong".into())),
            serde_json::json!("pong")
        );
        assert_eq!(
            slot_value_to_json(&SlotValue::Bytes(vec![0xde, 0xad, 0xbe, 0xef])),
            serde_json::json!("0xdeadbeef")
        );
        let nested = serde_json::json!({"a": [1, 2]});
        assert_eq!(slot_value_to_json(&SlotValue::Json(nested.clone())), nested);
    }

    #[test]
    fn from_event_extracts_node_emitted_and_ignores_others() {
        let run = RunId::new();
        let node = NodeId::new("dev.starter.agent").expect("valid node id");
        let emitted = FlowEvent::NodeEmitted {
            run,
            node: node.clone(),
            slot: "out".into(),
            value: SlotValue::String("pong".into()),
        };
        let dto = NodeSlotValue::from_event(&emitted).expect("expected Some");
        assert_eq!(dto.run, run);
        assert_eq!(dto.node, node);
        assert_eq!(dto.slot, "out");
        assert_eq!(dto.value, serde_json::json!("pong"));

        let started = FlowEvent::RunStarted {
            run,
            flow: FlowId::new("dev.starter.flow").expect("valid flow id"),
        };
        assert!(NodeSlotValue::from_event(&started).is_none());
    }

    #[test]
    fn node_slot_value_round_trips_through_serde() {
        let dto = NodeSlotValue {
            run: RunId::new(),
            node: NodeId::new("dev.starter.trigger").expect("valid node id"),
            slot: "fire".into(),
            value: serde_json::json!({"payload": "ping"}),
        };
        let s = serde_json::to_string(&dto).unwrap();
        let back: NodeSlotValue = serde_json::from_str(&s).unwrap();
        assert_eq!(back, dto);
    }
}
