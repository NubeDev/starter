//! `rubix.flow_ops.duplicate` — tool dispatch.
//!
//! Reads the latest live revision of `source_flow_id` from the
//! shared [`FlowDefStore`], rewrites the YAML body's `id:` field
//! to `target_flow_id`, and writes a fresh revision under the new
//! id. The target must not already have a live revision; this
//! verb refuses to overwrite.
//!
//! The successful response carries a [`Diagnostic`] keyed
//! `rubix.flow.duplicated`. The companion `change_for` impl
//! produces a `Op::Create` [`ChangeDraft`] whose `after` payload
//! is the [`FlowDefChange`] snapshot for the new revision — the
//! undo dispatcher walks the duplicate back through
//! [`super::store::FlowDefReversible`].
//!
//! See [docs/design/flows/](../../../../docs/design/flows/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::flow_ops::duplicate::{FlowDuplicateRequest, FlowDuplicateResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::flow_ops::store::{FlowDefChange, FlowDefStore, FLOW_DEFINITION_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.flow_ops.duplicate`.
pub struct FlowDuplicateTool {
    store: Arc<dyn FlowDefStore>,
}

impl FlowDuplicateTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn FlowDefStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for FlowDuplicateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.flow_ops.duplicate".to_owned(),
            description: rubix_spi::dto::flow_ops::duplicate::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source_flow_id": { "type": "string", "minLength": 1 },
                    "target_flow_id": { "type": "string", "minLength": 1 }
                },
                "required": ["source_flow_id", "target_flow_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: FlowDuplicateRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("FlowDuplicateRequest: {e}"),
            })?;

        if req.source_flow_id == req.target_flow_id {
            return Err(Error::Invalid {
                message: "source_flow_id and target_flow_id must differ".to_owned(),
            });
        }

        // Refuse to overwrite a live target.
        if self
            .store
            .fetch_latest_live(&req.target_flow_id)
            .await?
            .is_some()
        {
            return Err(Error::Conflict {
                message: format!(
                    "flow {} already has a live revision; refuse to overwrite",
                    req.target_flow_id
                ),
            });
        }

        let source = self
            .store
            .fetch_latest_live(&req.source_flow_id)
            .await?
            .ok_or_else(|| Error::NotFound {
                what: format!("flow_definition flow_id:{}", req.source_flow_id),
            })?;

        // Rewrite the body's `id:` field to point at the new flow.
        // Cheap textual swap rather than a full re-emit: keeps
        // comments and ordering byte-stable. The YAML re-parses
        // through `parse_yaml` below to guarantee the rewrite is
        // structurally sound.
        let new_body = rewrite_flow_id(&source.body_yaml, &req.target_flow_id);
        let yaml = rubix_flows::parse_yaml(
            &format!("duplicate://{}", req.target_flow_id),
            new_body.as_bytes(),
        )
        .map_err(|e| Error::Internal {
            source: Box::new(std::io::Error::other(format!(
                "duplicate produced unparseable yaml: {e}"
            ))),
        })?;
        if yaml.id != req.target_flow_id {
            return Err(Error::Internal {
                source: Box::new(std::io::Error::other(
                    "duplicate rewrite did not update body id",
                )),
            });
        }

        let now_ms = now_epoch_ms();
        let (row, _prior) = self
            .store
            .insert_revision(&req.target_flow_id, &new_body, now_ms)
            .await?;

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.flow.duplicated").expect("hard-coded key parses"),
        )
        .with_param("source", DiagnosticParam::String(req.source_flow_id.clone()))
        .with_param("target", DiagnosticParam::String(req.target_flow_id.clone()))
        .with_param("at", DiagnosticParam::Timestamp(now_ms));

        let response = FlowDuplicateResponse {
            summary,
            source_flow_id: req.source_flow_id,
            target_flow_id: req.target_flow_id,
            revision_id: row.revision_id,
            created_at_ms: now_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for FlowDuplicateTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: FlowDuplicateResponse = serde_json::from_value(output.clone()).ok()?;
        // The duplicate writes the *first* revision under the new
        // flow_id, so there is no prior revision to un-supersede on
        // undo — `prior_revision_id` is always `None`.
        let snap = FlowDefChange {
            flow_id: resp.target_flow_id.clone(),
            revision_id: resp.revision_id.clone(),
            prior_revision_id: None,
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

/// Replace the first `id: <something>` line with `id: <target>`.
/// The bundled YAMLs declare `id:` on the first non-comment line,
/// so a line-based swap is enough; if no match is found the body
/// is returned unchanged and the post-parse cross-check above
/// catches it.
fn rewrite_flow_id(body: &str, target: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut replaced = false;
    for line in body.lines() {
        if !replaced {
            let trimmed = line.trim_start();
            if trimmed.starts_with("id:") || trimmed.starts_with("id :") {
                out.push_str(&format!("id: {target}"));
                out.push('\n');
                replaced = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
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

    const SRC_YAML: &str = "id: com.x.src\ntrigger: explicit\nnodes:\n  - id: agent\n    kind: ai-agent\n    config: {}\nlinks: []\n";

    #[tokio::test]
    async fn duplicate_copies_latest_revision_under_new_id() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        store.insert_revision("com.x.src", SRC_YAML, 1).await.unwrap();
        let tool = FlowDuplicateTool::new(store.clone());
        let out = tool
            .invoke(serde_json::json!({
                "source_flow_id": "com.x.src",
                "target_flow_id": "com.x.dst",
            }))
            .await
            .unwrap();
        let resp: FlowDuplicateResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.flow.duplicated");
        let new = store.fetch_latest_live("com.x.dst").await.unwrap().unwrap();
        assert!(new.body_yaml.contains("id: com.x.dst"));
        assert!(!new.body_yaml.contains("id: com.x.src"));
    }

    #[tokio::test]
    async fn duplicate_refuses_overwrite() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        store.insert_revision("com.x.src", SRC_YAML, 1).await.unwrap();
        let dst = "id: com.x.dst\ntrigger: explicit\nnodes:\n  - id: agent\n    kind: ai-agent\n    config: {}\nlinks: []\n";
        store.insert_revision("com.x.dst", dst, 2).await.unwrap();
        let tool = FlowDuplicateTool::new(store);
        let err = tool
            .invoke(serde_json::json!({
                "source_flow_id": "com.x.src",
                "target_flow_id": "com.x.dst",
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Conflict { .. }));
    }

    #[tokio::test]
    async fn duplicate_missing_source_is_not_found() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        let tool = FlowDuplicateTool::new(store);
        let err = tool
            .invoke(serde_json::json!({
                "source_flow_id": "com.x.ghost",
                "target_flow_id": "com.x.dst",
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn change_for_returns_create_with_no_prior() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        store.insert_revision("com.x.src", SRC_YAML, 1).await.unwrap();
        let tool = FlowDuplicateTool::new(store);
        let input = serde_json::json!({
            "source_flow_id": "com.x.src",
            "target_flow_id": "com.x.dst",
        });
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        let snap: FlowDefChange = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(snap.flow_id, "com.x.dst");
        assert!(snap.prior_revision_id.is_none());
    }
}
