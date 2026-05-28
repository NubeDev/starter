//! `rubix.team.unassign` — tool dispatch.
//!
//! Inverse of [`crate::team::assign`]. Same idempotency posture
//! (no-op returns a diagnostic but no `ChangeDraft`), same
//! patch-shaped snapshot (`members` only), same out-of-band
//! stash of `_prior_members` / `_new_members` on the JSON for
//! `change_for` to consume.
//!
//! See [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::team::unassign::{TeamUnassignRequest, TeamUnassignResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::team::store::{TeamAdminStore, TeamPatch, TEAM_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.team.unassign`.
pub struct TeamUnassignTool {
    store: Arc<dyn TeamAdminStore>,
}

impl TeamUnassignTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TeamAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TeamUnassignTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.team.unassign".to_owned(),
            description: rubix_spi::dto::team::unassign::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "team_id": { "type": "string", "minLength": 1 },
                    "user_id": { "type": "string", "minLength": 1 }
                },
                "required": ["team_id", "user_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: TeamUnassignRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("TeamUnassignRequest: {e}"),
            })?;
        if req.team_id.is_empty() || req.team_id.trim() != req.team_id {
            return Err(Error::Invalid {
                message: "TeamUnassignRequest.team_id must be non-empty and trimmed".to_owned(),
            });
        }
        if req.user_id.is_empty() || req.user_id.trim() != req.user_id {
            return Err(Error::Invalid {
                message: "TeamUnassignRequest.user_id must be non-empty and trimmed".to_owned(),
            });
        }

        let now_ms = now_epoch_ms();
        let (prior, new) = self.store.unassign(&req.team_id, &req.user_id).await?;
        let already_not_member = !prior.members.contains_key(&req.user_id);

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.team.unassigned").expect("hard-coded key parses"),
        )
        .with_param("team", DiagnosticParam::String(new.name.clone()))
        .with_param("user", DiagnosticParam::String(req.user_id.clone()))
        .with_param("at", DiagnosticParam::Timestamp(now_ms));

        let response = TeamUnassignResponse {
            summary,
            team_id: new.team_id.clone(),
            user_id: req.user_id.clone(),
            already_not_member,
            unassigned_at_ms: now_ms,
        };
        // Mirror the assign verb's pattern: stash the membership
        // patches out-of-band on the JSON so change_for can
        // rebuild the sparse `TeamPatch` without round-tripping
        // through the store. See team/assign.rs for the rationale.
        let mut value = serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;
        if !already_not_member {
            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "_prior_members".to_owned(),
                    serde_json::to_value(&prior.members).unwrap_or(Value::Null),
                );
                obj.insert(
                    "_new_members".to_owned(),
                    serde_json::to_value(&new.members).unwrap_or(Value::Null),
                );
            }
        }
        Ok(value)
    }
}

impl ReversibleTool for TeamUnassignTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let obj = output.as_object()?;
        if obj
            .get("already_not_member")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            // Idempotent no-op — never record a Change.
            return None;
        }
        let team_id = obj.get("team_id")?.as_str()?.to_owned();
        let prior_members = obj.get("_prior_members")?.clone();
        let new_members = obj.get("_new_members")?.clone();

        let before = TeamPatch {
            members: serde_json::from_value(prior_members).ok(),
            name: None,
            description: None,
        };
        let after = TeamPatch {
            members: serde_json::from_value(new_members).ok(),
            name: None,
            description: None,
        };

        Some(ChangeDraft::update(
            ResourceRef {
                kind: TEAM_KIND.into(),
                id: Some(team_id),
                owner: None,
                tenant: None,
            },
            serde_json::to_value(&before).ok()?,
            serde_json::to_value(&after).ok()?,
        ))
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
    use crate::team::store::{InMemoryTeamStore, TeamReversible, TeamRow};
    use serde_json::json;
    use starter_spi::changelog::{Actor, Change, ChangeId, GroupId, Reversible};
    use std::collections::BTreeMap;

    async fn seeded_with_member() -> Arc<InMemoryTeamStore> {
        let store = Arc::new(InMemoryTeamStore::new());
        store
            .create(TeamRow {
                team_id: "t-1".into(),
                name: "Ops".into(),
                description: None,
                members: BTreeMap::new(),
            })
            .await
            .unwrap();
        store.assign("t-1", "u-1", 100).await.unwrap();
        store.assign("t-1", "u-2", 200).await.unwrap();
        store
    }

    #[tokio::test]
    async fn unassign_removes_member_and_records_patch_draft() {
        let store = seeded_with_member().await;
        let tool = TeamUnassignTool::new(store.clone());
        let input = json!({"team_id": "t-1", "user_id": "u-1"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: TeamUnassignResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.team.unassigned");
        assert!(!resp.already_not_member);

        let row = store.get("t-1").await.unwrap().unwrap();
        assert!(!row.members.contains_key("u-1"));
        assert!(row.members.contains_key("u-2"), "other members preserved");

        let draft = tool.change_for(&input, &out).expect("draft");
        let before: TeamPatch = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: TeamPatch = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert!(before.members.as_ref().unwrap().contains_key("u-1"));
        assert!(!after.members.as_ref().unwrap().contains_key("u-1"));
        assert!(
            after.members.as_ref().unwrap().contains_key("u-2"),
            "patch carries the full new map, not a diff",
        );
        assert!(before.name.is_none() && after.name.is_none());
    }

    #[tokio::test]
    async fn unassign_non_member_is_idempotent_and_skips_draft() {
        let store = seeded_with_member().await;
        let tool = TeamUnassignTool::new(store);
        let input = json!({"team_id": "t-1", "user_id": "u-ghost"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: TeamUnassignResponse = serde_json::from_value(out.clone()).unwrap();
        assert!(resp.already_not_member);
        assert!(tool.change_for(&input, &out).is_none());
    }

    #[tokio::test]
    async fn second_unassign_is_idempotent_and_skips_draft() {
        let store = seeded_with_member().await;
        let tool = TeamUnassignTool::new(store);
        let input = json!({"team_id": "t-1", "user_id": "u-1"});
        let _ = tool.invoke(input.clone()).await.unwrap();
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: TeamUnassignResponse = serde_json::from_value(out.clone()).unwrap();
        assert!(resp.already_not_member);
        assert!(tool.change_for(&input, &out).is_none());
    }

    #[tokio::test]
    async fn missing_team_returns_not_found() {
        let store = Arc::new(InMemoryTeamStore::new());
        let tool = TeamUnassignTool::new(store);
        let err = tool
            .invoke(json!({"team_id": "t-ghost", "user_id": "u-1"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn empty_ids_are_rejected() {
        let store = seeded_with_member().await;
        let tool = TeamUnassignTool::new(store);
        let err = tool
            .invoke(json!({"team_id": "", "user_id": "u-1"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
        let store = seeded_with_member().await;
        let tool = TeamUnassignTool::new(store);
        let err = tool
            .invoke(json!({"team_id": "t-1", "user_id": ""}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn reversible_round_trip_restores_member_with_original_timestamp() {
        let store = seeded_with_member().await;
        let tool = TeamUnassignTool::new(store.clone());
        let input = json!({"team_id": "t-1", "user_id": "u-1"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft");
        assert!(!store
            .get("t-1")
            .await
            .unwrap()
            .unwrap()
            .members
            .contains_key("u-1"));

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

        let row = store.get("t-1").await.unwrap().unwrap();
        assert_eq!(
            row.members.get("u-1"),
            Some(&100),
            "undo restores the original assigned_at timestamp"
        );
        assert_eq!(row.members.get("u-2"), Some(&200));
    }

    #[tokio::test]
    async fn undo_preserves_concurrent_rename() {
        // Patch-shape contract: unassign records only `members`,
        // so undoing it must not clobber a concurrent name flip.
        let store = seeded_with_member().await;
        let tool = TeamUnassignTool::new(store.clone());
        let input = json!({"team_id": "t-1", "user_id": "u-1"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft");

        // Concurrent rename between the unassign and its undo.
        let mut row = store.get("t-1").await.unwrap().unwrap();
        row.name = "Operations".to_owned();
        store.put(row).await.unwrap();

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

        let row = store.get("t-1").await.unwrap().unwrap();
        assert_eq!(
            row.name, "Operations",
            "concurrent rename must survive the unassign undo",
        );
        assert_eq!(row.members.get("u-1"), Some(&100));
    }
}
