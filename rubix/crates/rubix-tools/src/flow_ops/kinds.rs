//! `rubix.flow_ops.kinds` — tool dispatch.
//!
//! Read-only verb: returns the list of registered node kinds as
//! [`FlowKindItem`]s sorted by `kind_id`. The tool holds a snapshot
//! of the [`NodeKindRegistry`] contents (kind_id + config_schema +
//! default_label) captured at boot — no async lookup on the hot
//! path. See [docs/design/flows/](../../../../docs/design/flows/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::flow_ops::kinds::{FlowKindItem, FlowKindsRequest, FlowKindsResponse};
use serde_json::Value;
use starter_flow_spi::node::NodeBehavior;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

/// Concrete [`Tool`] for `rubix.flow_ops.kinds`.
pub struct FlowKindsTool {
    kinds: Vec<FlowKindItem>,
}

impl FlowKindsTool {
    /// Construct from a pre-built kind list (typically what the boot
    /// wiring builds from the [`NodeKindRegistry`]).
    pub fn new(kinds: Vec<FlowKindItem>) -> Self {
        let mut kinds = kinds;
        kinds.sort_by(|a, b| a.kind_id.cmp(&b.kind_id));
        Self { kinds }
    }

    /// Convenience constructor: snapshot a slice of behaviours into
    /// [`FlowKindItem`]s by calling `kind_id()` and `config_schema()`
    /// on each and deriving `default_label` from the last
    /// reverse-DNS segment ("starter.flow.counter" → "Counter").
    pub fn from_behaviors(behaviors: &[Arc<dyn NodeBehavior>]) -> Self {
        let kinds: Vec<FlowKindItem> = behaviors
            .iter()
            .map(|b| {
                let kind_id = b.kind_id().as_str().to_owned();
                let schema = serde_json::to_value(b.config_schema())
                    .unwrap_or(Value::Object(Default::default()));
                let default_label = default_label_for(&kind_id);
                FlowKindItem {
                    kind_id,
                    config_schema: schema,
                    default_label,
                }
            })
            .collect();
        Self::new(kinds)
    }
}

/// Title-case the final reverse-DNS segment as a presentation-layer
/// fallback. Pure string transform; no i18n.
fn default_label_for(kind_id: &str) -> String {
    let tail = kind_id.rsplit('.').next().unwrap_or(kind_id);
    let mut out = String::with_capacity(tail.len());
    let mut next_upper = true;
    for ch in tail.chars() {
        if ch == '_' || ch == '-' {
            out.push(' ');
            next_upper = true;
        } else if next_upper {
            for u in ch.to_uppercase() {
                out.push(u);
            }
            next_upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

#[async_trait]
impl Tool for FlowKindsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.flow_ops.kinds".to_owned(),
            description: rubix_spi::dto::flow_ops::kinds::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let _req: FlowKindsRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("FlowKindsRequest: {e}"),
        })?;
        let count = self.kinds.len();
        let summary = Diagnostic::new(
            MessageKey::parse("rubix.flow.kinds.listed").expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));
        let response = FlowKindsResponse {
            summary,
            count,
            kinds: self.kinds.clone(),
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_label_title_cases_last_segment() {
        assert_eq!(default_label_for("starter.flow.counter"), "Counter");
        assert_eq!(
            default_label_for("starter.flow.trigger_schedule"),
            "Trigger Schedule"
        );
        assert_eq!(default_label_for("com.acme.http-out"), "Http Out");
    }

    #[tokio::test]
    async fn empty_registry_lists_zero_kinds() {
        let tool = FlowKindsTool::new(vec![]);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: FlowKindsResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.flow.kinds.listed");
        assert_eq!(resp.count, 0);
    }

    #[tokio::test]
    async fn kinds_come_back_sorted_by_kind_id() {
        let tool = FlowKindsTool::new(vec![
            FlowKindItem {
                kind_id: "starter.flow.zed".into(),
                config_schema: serde_json::json!({}),
                default_label: "Zed".into(),
            },
            FlowKindItem {
                kind_id: "starter.flow.ada".into(),
                config_schema: serde_json::json!({}),
                default_label: "Ada".into(),
            },
        ]);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: FlowKindsResponse = serde_json::from_value(out).unwrap();
        let ids: Vec<&str> = resp.kinds.iter().map(|k| k.kind_id.as_str()).collect();
        assert_eq!(ids, vec!["starter.flow.ada", "starter.flow.zed"]);
    }

    #[tokio::test]
    async fn from_behaviors_snapshots_kind_id_schema_and_label() {
        use starter_flow_nodes::counter::Counter;
        let behaviors: Vec<Arc<dyn NodeBehavior>> = vec![Arc::new(Counter::new())];
        let tool = FlowKindsTool::from_behaviors(&behaviors);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: FlowKindsResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 1);
        let entry = &resp.kinds[0];
        assert_eq!(entry.kind_id, "starter.flow.counter");
        assert_eq!(entry.default_label, "Counter");
        assert!(
            entry.config_schema.is_object(),
            "config_schema must serialise as a JSON object"
        );
    }
}
