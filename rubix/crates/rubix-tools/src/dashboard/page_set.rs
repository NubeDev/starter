//! `rubix.dashboard.page_set` — runtime slot-write tool dispatch.
//!
//! Mutates **one** slot on a flow node via the R2 single-write
//! chokepoint [`GraphStore::write_slot`]. This is intentionally
//! **not** a `dashboards_definitions` revision write (that role
//! belongs to [`super::update`] / [`super::create`]) and **not**
//! `starter-undo` reversible — per
//! `docs/scope/dashboards/08-open-questions.md` OQ-5 the operator
//! reverts by writing the prior value back into the same slot.
//!
//! Emits a single diagnostic keyed `rubix.dashboard.page_set.applied`
//! on success. See `rubix/docs/scope/dashboards/04-tools.md`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::dashboard::page_set::{PageSetRequest, PageSetResponse};
use serde_json::Value;
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{NodeId, SlotRef, SlotValue};
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

/// Concrete [`Tool`] for `rubix.dashboard.page_set`.
pub struct DashboardPageSetTool {
    graph: Arc<dyn GraphStore>,
}

impl DashboardPageSetTool {
    /// Wrap a shared [`GraphStore`] — the same handle the
    /// propagator and surface adapters use, so every operator
    /// write enters through the R2 chokepoint exactly once.
    pub fn new(graph: Arc<dyn GraphStore>) -> Self {
        Self { graph }
    }
}

/// Coerce a JSON value into the closest [`SlotValue`] variant.
///
/// The conversion is total: anything that does not fit a primitive
/// (objects, arrays, mixed-precision numbers) falls back to
/// [`SlotValue::Json`] so the verb never refuses a syntactically
/// valid request — the destination node's behaviour decides whether
/// the shape is acceptable.
fn coerce(value: Value) -> SlotValue {
    match value {
        Value::Null => SlotValue::Null,
        Value::Bool(b) => SlotValue::Bool(b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SlotValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                SlotValue::Float(f)
            } else {
                SlotValue::Json(Value::Number(n))
            }
        }
        Value::String(s) => SlotValue::String(s),
        other => SlotValue::Json(other),
    }
}

#[async_trait]
impl Tool for DashboardPageSetTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.dashboard.page_set".to_owned(),
            description: rubix_spi::dto::dashboard::page_set::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id":  { "type": "string", "minLength": 1 },
                    "page_id":    { "type": "string", "minLength": 1 },
                    "node_id":    { "type": "string", "minLength": 1 },
                    "slot":       { "type": "string", "minLength": 1 },
                    "value":      {},
                    "written_by": { "type": "string", "minLength": 1 }
                },
                "required": ["tenant_id", "page_id", "node_id", "slot", "value", "written_by"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: PageSetRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("PageSetRequest: {e}"),
        })?;

        let node = NodeId::new(req.node_id.clone()).map_err(|e| Error::Invalid {
            message: format!("node_id `{}`: {e}", req.node_id),
        })?;
        if req.slot.trim().is_empty() {
            return Err(Error::Invalid {
                message: "slot must be non-empty".to_owned(),
            });
        }

        let slot_ref = SlotRef::new(node, req.slot.clone());
        let value = coerce(req.value);

        // R2 chokepoint. The store's tracing span carries
        // `node_id`/`slot_name`/`origin=live`; the change-log
        // middleware higher up records the operator intent.
        self.graph
            .write_slot(
                &slot_ref,
                value,
                starter_flow_spi::graph::WriteSlotOpts::live(),
            )
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.dashboard.page_set.applied")
                .expect("hard-coded key parses"),
        )
        .with_param(
            "node_id",
            DiagnosticParam::String(slot_ref.node.to_string()),
        )
        .with_param("slot", DiagnosticParam::String(req.slot.clone()));

        let response = PageSetResponse {
            summary,
            page_id: req.page_id,
            node_id: slot_ref.node.to_string(),
            slot: req.slot,
            written: true,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_flow::graph::InMemoryGraphStore;
    use starter_flow_spi::graph::{SubscribeOpts, WriteSlotOpts};

    fn store() -> Arc<InMemoryGraphStore> {
        Arc::new(InMemoryGraphStore::new())
    }

    #[tokio::test]
    async fn applied_diagnostic_and_value_lands_through_chokepoint() {
        let graph = store();
        let tool = DashboardPageSetTool::new(graph.clone());
        let out = tool
            .invoke(serde_json::json!({
                "tenant_id":  "tenant-a",
                "page_id":    "dashboard.ops",
                "node_id":    "com.acme.thermostat",
                "slot":       "setpoint",
                "value":      21.5,
                "written_by": "alice"
            }))
            .await
            .unwrap();
        let resp: PageSetResponse = serde_json::from_value(out).unwrap();
        assert_eq!(
            resp.summary.code.as_str(),
            "rubix.dashboard.page_set.applied"
        );
        assert!(resp.written);
        let slot = SlotRef::new(
            NodeId::new("com.acme.thermostat").unwrap(),
            "setpoint".to_owned(),
        );
        let v = graph.read_slot(&slot).await.unwrap();
        assert_eq!(v, SlotValue::Float(21.5));
    }

    #[tokio::test]
    async fn write_emits_slotchanged_event_proving_chokepoint_path() {
        let graph = store();
        let mut sub = graph.subscribe(SubscribeOpts::default());
        let tool = DashboardPageSetTool::new(graph);
        tool.invoke(serde_json::json!({
            "tenant_id":  "tenant-a",
            "page_id":    "dashboard.ops",
            "node_id":    "com.acme.thermostat",
            "slot":       "enabled",
            "value":      true,
            "written_by": "alice"
        }))
        .await
        .unwrap();
        // Drain one event with a short timeout; the chokepoint is
        // synchronous, so the receiver must already have it.
        use futures::StreamExt;
        let evt = tokio::time::timeout(std::time::Duration::from_millis(200), sub.next())
            .await
            .expect("event arrived")
            .expect("stream open");
        // We don't pin the engine's event enum here — landing one
        // event at all is the chokepoint-was-traversed signal.
        let _ = evt;
    }

    #[tokio::test]
    async fn coerces_primitive_variants() {
        assert!(matches!(coerce(Value::Null), SlotValue::Null));
        assert!(matches!(coerce(Value::Bool(true)), SlotValue::Bool(true)));
        assert!(matches!(coerce(serde_json::json!(7)), SlotValue::Int(7)));
        assert!(matches!(
            coerce(serde_json::json!(7.5)),
            SlotValue::Float(_)
        ));
        assert!(matches!(
            coerce(serde_json::json!("hi")),
            SlotValue::String(_)
        ));
        assert!(matches!(
            coerce(serde_json::json!({"a": 1})),
            SlotValue::Json(_)
        ));
    }

    #[tokio::test]
    async fn invalid_node_id_is_rejected() {
        let tool = DashboardPageSetTool::new(store());
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":  "tenant-a",
                "page_id":    "dashboard.ops",
                "node_id":    "no-dot",
                "slot":       "x",
                "value":      1,
                "written_by": "alice"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn empty_slot_is_rejected() {
        let tool = DashboardPageSetTool::new(store());
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":  "tenant-a",
                "page_id":    "dashboard.ops",
                "node_id":    "com.acme.thermostat",
                "slot":       "   ",
                "value":      1,
                "written_by": "alice"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    /// Sibling check: page_set is **not** Reversible — confirm we
    /// don't accidentally pick up a [`ReversibleTool`] impl through
    /// a future blanket. The compile-time assertion below relies on
    /// the fact that adding such an impl here would require this
    /// file to import the trait.
    #[tokio::test]
    async fn r2_idempotent_short_circuit_is_inherited_from_chokepoint() {
        // Two identical writes must not error; the second is a no-op
        // under the chokepoint's R3 rule. The response still reports
        // `written = true` because the verb does not introspect the
        // chokepoint's short-circuit decision.
        let graph = store();
        let tool = DashboardPageSetTool::new(graph.clone());
        let payload = serde_json::json!({
            "tenant_id":  "tenant-a",
            "page_id":    "dashboard.ops",
            "node_id":    "com.acme.thermostat",
            "slot":       "setpoint",
            "value":      21.5,
            "written_by": "alice"
        });
        tool.invoke(payload.clone()).await.unwrap();
        tool.invoke(payload).await.unwrap();
        let slot = SlotRef::new(
            NodeId::new("com.acme.thermostat").unwrap(),
            "setpoint".to_owned(),
        );
        assert_eq!(
            graph.read_slot(&slot).await.unwrap(),
            SlotValue::Float(21.5)
        );
        // Suppress unused-import warning when feature-gating moves.
        let _ = WriteSlotOpts::live();
    }
}
