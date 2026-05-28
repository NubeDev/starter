//! `rubix.team.delete` — tool dispatch.
//!
//! Hard-deletes a team row including its membership map. Cascade
//! decision documented in the DTO module doc: members ride on the
//! snapshot so undo restores the team byte-exact.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::team::delete::{TeamDeleteRequest, TeamDeleteResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::team::store::{TeamAdminStore, TeamRow, TEAM_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.team.delete`.
pub struct TeamDeleteTool {
    store: Arc<dyn TeamAdminStore>,
}

impl TeamDeleteTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TeamAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TeamDeleteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.team.delete".to_owned(),
            description: rubix_spi::dto::team::delete::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "team_id": { "type": "string", "minLength": 1 }
                },
                "required": ["team_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: TeamDeleteRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("TeamDeleteRequest: {e}"),
        })?;

        if req.team_id.is_empty() || req.team_id.trim() != req.team_id {
            return Err(Error::Invalid {
                message: "TeamDeleteRequest.team_id must be non-empty and trimmed".to_owned(),
            });
        }

        // Snapshot-first: load the prior row so we can echo every
        // identity-bearing field on the response and produce a
        // byte-exact `before` snapshot in `change_for` — without
        // a follow-up store read (proposal §3.1 fix).
        let prior =
            self.store
                .get(&req.team_id)
                .await?
                .ok_or_else(|| Error::NotFound {
                    what: format!("team:{}", req.team_id),
                })?;

        self.store.delete(&req.team_id).await?;
        let deleted_at_ms = now_epoch_ms();
        let member_count = prior.members.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.team.deleted").expect("hard-coded key parses"),
        )
        .with_param("team", DiagnosticParam::String(prior.team_id.clone()))
        .with_param("name", DiagnosticParam::String(prior.name.clone()))
        .with_param("members", DiagnosticParam::I64(member_count as i64))
        .with_param("at", DiagnosticParam::Timestamp(deleted_at_ms));

        let response = TeamDeleteResponse {
            summary,
            team_id: prior.team_id,
            name: prior.name,
            description: prior.description,
            members: prior.members,
            member_count,
            deleted_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for TeamDeleteTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: TeamDeleteResponse = serde_json::from_value(output.clone()).ok()?;
        let before = TeamRow {
            team_id: resp.team_id.clone(),
            name: resp.name,
            description: resp.description,
            members: resp.members,
        };
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: TEAM_KIND.into(),
                id: Some(resp.team_id),
                owner: None,
                tenant: None,
            },
            op: Op::Delete,
            before: Some(serde_json::to_value(&before).ok()?),
            after: None,
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
    use serde_json::json;
    use starter_spi::changelog::Reversible;

    #[tokio::test]
    async fn delete_empty_team_succeeds_and_emits_zero_member_count() {
        let store = Arc::new(InMemoryTeamStore::new());
        store
            .create(TeamRow {
                team_id: "t-1".into(),
                name: "Ops".into(),
                description: None,
                members: Default::default(),
            })
            .await
            .unwrap();
        let tool = TeamDeleteTool::new(store.clone());
        let out = tool.invoke(json!({"team_id": "t-1"})).await.unwrap();
        let resp: TeamDeleteResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.team.deleted");
        assert_eq!(resp.member_count, 0);
        assert!(store.get("t-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_team_with_members_cascades_and_records_count() {
        let store = Arc::new(InMemoryTeamStore::new());
        store
            .create(TeamRow {
                team_id: "t-1".into(),
                name: "Ops".into(),
                description: Some("desc".into()),
                members: Default::default(),
            })
            .await
            .unwrap();
        store.assign("t-1", "u-1", 100).await.unwrap();
        store.assign("t-1", "u-2", 200).await.unwrap();
        let tool = TeamDeleteTool::new(store.clone());
        let out = tool.invoke(json!({"team_id": "t-1"})).await.unwrap();
        let resp: TeamDeleteResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.member_count, 2);
        assert_eq!(resp.members.get("u-1"), Some(&100));
        assert_eq!(resp.members.get("u-2"), Some(&200));
        assert!(store.get("t-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn missing_team_returns_not_found() {
        let store = Arc::new(InMemoryTeamStore::new());
        let tool = TeamDeleteTool::new(store);
        let err = tool
            .invoke(json!({"team_id": "t-ghost"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn empty_team_id_is_rejected() {
        let store = Arc::new(InMemoryTeamStore::new());
        let tool = TeamDeleteTool::new(store);
        let err = tool.invoke(json!({"team_id": ""})).await.unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn change_for_records_delete_with_full_before_snapshot() {
        let store = Arc::new(InMemoryTeamStore::new());
        store
            .create(TeamRow {
                team_id: "t-1".into(),
                name: "Ops".into(),
                description: Some("desc".into()),
                members: Default::default(),
            })
            .await
            .unwrap();
        store.assign("t-1", "u-1", 100).await.unwrap();
        let tool = TeamDeleteTool::new(store);
        let input = json!({"team_id": "t-1"});
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft");
        assert!(matches!(draft.op, Op::Delete));
        assert!(draft.after.is_none());
        let before: TeamRow =
            serde_json::from_value(draft.before.unwrap()).unwrap();
        assert_eq!(before.team_id, "t-1");
        assert_eq!(before.name, "Ops");
        assert_eq!(before.description.as_deref(), Some("desc"));
        assert_eq!(before.members.get("u-1"), Some(&100));
    }

    #[tokio::test]
    async fn reversible_round_trip_restores_team_with_members() {
        use crate::team::store::TeamReversible;
        use starter_spi::changelog::{Actor, Change, ChangeId, GroupId};
        let store = Arc::new(InMemoryTeamStore::new());
        store
            .create(TeamRow {
                team_id: "t-1".into(),
                name: "Ops".into(),
                description: Some("on-call".into()),
                members: Default::default(),
            })
            .await
            .unwrap();
        store.assign("t-1", "u-1", 100).await.unwrap();
        store.assign("t-1", "u-2", 200).await.unwrap();

        let tool = TeamDeleteTool::new(store.clone());
        let input = json!({"team_id": "t-1"});
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft");
        assert!(store.get("t-1").await.unwrap().is_none());

        let change = Change {
            id: ChangeId("c-test".into()),
            group_id: GroupId("g-test".into()),
            at: chrono::Utc::now(),
            actor: Actor::System,
            resource: draft.resource.clone(),
            op: draft.op.clone(),
            before: draft.before.clone(),
            after: draft.after.clone(),
            resource_version: None,
            correlation: None,
            patch: None,
        };
        let reversible = TeamReversible::new(store.clone());
        reversible.apply_inverse(&change).await.unwrap();

        let restored = store.get("t-1").await.unwrap().expect("restored");
        assert_eq!(restored.name, "Ops");
        assert_eq!(restored.description.as_deref(), Some("on-call"));
        assert_eq!(restored.members.get("u-1"), Some(&100));
        assert_eq!(restored.members.get("u-2"), Some(&200));
    }
}
