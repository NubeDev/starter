//! `rubix.team.create` — tool dispatch.
//!
//! Provisions a new team via the shared [`TeamAdminStore`]. The
//! response carries a `Diagnostic` keyed `rubix.team.created`.
//! Snapshot shape: `Op::Create`, `after` = the new [`TeamRow`] JSON
//! (with an empty members map). See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::team::create::{TeamCreateRequest, TeamCreateResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;
use uuid::Uuid;

use crate::team::store::{TeamAdminStore, TeamRow, TEAM_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.team.create`.
pub struct TeamCreateTool {
    store: Arc<dyn TeamAdminStore>,
}

impl TeamCreateTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TeamAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TeamCreateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.team.create".to_owned(),
            description: rubix_spi::dto::team::create::DESCRIPTOR.purpose.to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "minLength": 1 },
                    "description": { "type": ["string", "null"] }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: TeamCreateRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("TeamCreateRequest: {e}"),
            })?;
        let name = req.name.trim();
        if name.is_empty() {
            return Err(Error::Invalid {
                message: "team name must be non-empty".to_owned(),
            });
        }
        let team_id = format!("t-{}", Uuid::new_v4().simple());
        let created_at_ms = now_epoch_ms();
        let row = TeamRow {
            team_id: team_id.clone(),
            name: name.to_owned(),
            description: req.description.clone(),
            members: BTreeMap::new(),
        };
        let row = self.store.create(row).await?;

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.team.created").expect("hard-coded key parses"),
        )
        .with_param("name", DiagnosticParam::String(row.name.clone()))
        .with_param("at", DiagnosticParam::Timestamp(created_at_ms));

        let response = TeamCreateResponse {
            summary,
            team_id: row.team_id,
            name: row.name,
            created_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for TeamCreateTool {
    fn change_for(&self, input: &Value, output: &Value) -> Option<ChangeDraft> {
        let req: TeamCreateRequest = serde_json::from_value(input.clone()).ok()?;
        let resp: TeamCreateResponse = serde_json::from_value(output.clone()).ok()?;
        let row = TeamRow {
            team_id: resp.team_id.clone(),
            name: resp.name,
            description: req.description,
            members: BTreeMap::new(),
        };
        let after = serde_json::to_value(&row).ok()?;
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: TEAM_KIND.into(),
                id: Some(row.team_id),
                owner: None,
                tenant: None,
            },
            op: Op::Create,
            before: None,
            after: Some(after),
            resource_version: None,
            correlation: None,
        })
    }
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
    use crate::team::store::InMemoryTeamStore;

    #[tokio::test]
    async fn create_emits_created_diagnostic_and_persists_row() {
        let store = Arc::new(InMemoryTeamStore::new());
        let tool = TeamCreateTool::new(store.clone());
        let out = tool
            .invoke(serde_json::json!({"name": "Ops"}))
            .await
            .unwrap();
        let resp: TeamCreateResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.team.created");
        assert!(resp.team_id.starts_with("t-"));
        assert!(store.get(&resp.team_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn empty_name_is_rejected() {
        let tool = TeamCreateTool::new(Arc::new(InMemoryTeamStore::new()));
        let err = tool
            .invoke(serde_json::json!({"name": "   "}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn change_for_returns_create_draft_with_empty_members() {
        let store = Arc::new(InMemoryTeamStore::new());
        let tool = TeamCreateTool::new(store);
        let input = serde_json::json!({"name": "Ops"});
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert!(matches!(draft.op, Op::Create));
        let row: TeamRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert!(row.members.is_empty());
    }
}
