//! `rubix.flow_ops.deploy` — tool dispatch.
//!
//! Validates the YAML body through `rubix_flows::parse_yaml`,
//! cross-checks `flow_id` against the body's `id:`, and writes a
//! new row into the `flows_definitions` dimension table via the
//! shared [`FlowDefStore`]. The previously-live row (if any) is
//! marked superseded by the same call.
//!
//! The successful response carries a [`Diagnostic`] keyed
//! `rubix.flow.deployed`. The companion `change_for` impl produces
//! a `Op::Create` [`ChangeDraft`] whose `after` payload is the
//! [`FlowDefChange`] snapshot — the undo dispatcher walks the
//! deploy back through [`super::store::FlowDefReversible`].
//!
//! See [docs/design/flows/](../../../../docs/design/flows/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::flow_ops::deploy::{FlowDeployRequest, FlowDeployResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::flow_ops::store::{FlowDefChange, FlowDefStore, FLOW_DEFINITION_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.flow_ops.deploy`.
pub struct FlowDeployTool {
    store: Arc<dyn FlowDefStore>,
}

impl FlowDeployTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn FlowDefStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for FlowDeployTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.flow_ops.deploy".to_owned(),
            description: rubix_spi::dto::flow_ops::deploy::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "flow_id":   { "type": "string", "minLength": 1 },
                    "body_yaml": { "type": "string", "minLength": 1 }
                },
                "required": ["flow_id", "body_yaml"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: FlowDeployRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("FlowDeployRequest: {e}"),
        })?;

        let yaml = rubix_flows::parse_yaml(
            &format!("deploy://{}", req.flow_id),
            req.body_yaml.as_bytes(),
        )
        .map_err(|e| Error::Invalid {
            message: invalid_message(&format!("{e}")),
        })?;

        if yaml.id != req.flow_id {
            return Err(Error::Invalid {
                message: invalid_message(&format!(
                    "flow_id mismatch: request `{}` vs body `{}`",
                    req.flow_id, yaml.id
                )),
            });
        }

        let now_ms = now_epoch_ms();
        let (row, prior_revision_id) = self
            .store
            .insert_revision(&req.flow_id, &req.body_yaml, now_ms)
            .await?;

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.flow.deployed").expect("hard-coded key parses"),
        )
        .with_param("flow_id", DiagnosticParam::String(req.flow_id.clone()))
        .with_param("at", DiagnosticParam::Timestamp(now_ms));

        let response = FlowDeployResponse {
            summary,
            flow_id: req.flow_id,
            revision_id: row.revision_id,
            prior_revision_id,
            deployed_at_ms: now_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for FlowDeployTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: FlowDeployResponse = serde_json::from_value(output.clone()).ok()?;
        let snap = FlowDefChange {
            flow_id: resp.flow_id.clone(),
            revision_id: resp.revision_id.clone(),
            prior_revision_id: resp.prior_revision_id.clone(),
        };
        let after = serde_json::to_value(&snap).ok()?;
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: FLOW_DEFINITION_KIND.into(),
                id: Some(resp.revision_id),
                owner: None,
                tenant: None,
            },
            op: starter_spi::changelog::Op::Create,
            before: None,
            after: Some(after),
            resource_version: None,
            correlation: None,
        })
    }
}

fn invalid_message(detail: &str) -> String {
    let key = MessageKey::parse("rubix.flow.deploy.invalid").expect("hard-coded key parses");
    format!("{}: {detail}", key.as_str())
}

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow_ops::store::InMemoryFlowDefStore;

    const GOOD_YAML: &str = "id: com.x.a\ntrigger: explicit\nnodes:\n  - id: agent\n    kind: ai-agent\n    config: {}\nlinks: []\n";

    #[tokio::test]
    async fn first_deploy_emits_deployed_and_records_no_prior() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        let tool = FlowDeployTool::new(store.clone());
        let out = tool
            .invoke(serde_json::json!({"flow_id": "com.x.a", "body_yaml": GOOD_YAML}))
            .await
            .unwrap();
        let resp: FlowDeployResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.flow.deployed");
        assert!(resp.prior_revision_id.is_none());
        assert!(store.get(&resp.revision_id).is_some());
    }

    #[tokio::test]
    async fn second_deploy_supersedes_first() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        let tool = FlowDeployTool::new(store.clone());
        let out1 = tool
            .invoke(serde_json::json!({"flow_id": "com.x.a", "body_yaml": GOOD_YAML}))
            .await
            .unwrap();
        let r1: FlowDeployResponse = serde_json::from_value(out1).unwrap();
        let out2 = tool
            .invoke(serde_json::json!({"flow_id": "com.x.a", "body_yaml": GOOD_YAML}))
            .await
            .unwrap();
        let r2: FlowDeployResponse = serde_json::from_value(out2).unwrap();
        assert_eq!(
            r2.prior_revision_id.as_deref(),
            Some(r1.revision_id.as_str())
        );
    }

    #[tokio::test]
    async fn mismatched_flow_id_is_rejected_before_store() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        let tool = FlowDeployTool::new(store.clone());
        let err = tool
            .invoke(serde_json::json!({"flow_id": "com.y.b", "body_yaml": GOOD_YAML}))
            .await
            .unwrap_err();
        match err {
            Error::Invalid { message } => assert!(message.contains("rubix.flow.deploy.invalid")),
            other => panic!("expected Invalid, got {other:?}"),
        }
        assert!(store.is_empty(), "no row should land for a refused deploy");
    }

    #[tokio::test]
    async fn change_for_returns_create_with_flowdef_snapshot() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        let tool = FlowDeployTool::new(store);
        let input = serde_json::json!({"flow_id": "com.x.a", "body_yaml": GOOD_YAML});
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert_eq!(draft.resource.kind, FLOW_DEFINITION_KIND);
        let snap: FlowDefChange = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(snap.flow_id, "com.x.a");
    }
}
