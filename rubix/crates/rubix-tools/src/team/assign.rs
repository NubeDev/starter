//! `rubix.team.assign` — tool dispatch.
//!
//! Adds an existing user to an existing team via the shared
//! [`TeamAdminStore`]. The verb is idempotent — re-assigning an
//! existing member returns the same `rubix.team.assigned` code with
//! `already_member = true` and does **not** record a `ChangeDraft`.
//!
//! Snapshot shape: `Op::Update`, `before` / `after` carry the
//! sparse [`crate::team::store::TeamPatch`] with only the `members`
//! field populated. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::team::assign::{TeamAssignRequest, TeamAssignResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::team::store::{TeamAdminStore, TeamPatch, TEAM_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.team.assign`.
pub struct TeamAssignTool {
    store: Arc<dyn TeamAdminStore>,
}

impl TeamAssignTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TeamAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TeamAssignTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.team.assign".to_owned(),
            description: rubix_spi::dto::team::assign::DESCRIPTOR.purpose.to_owned(),
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
        let req: TeamAssignRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("TeamAssignRequest: {e}"),
        })?;
        let now_ms = now_epoch_ms();
        let (prior, new) = self
            .store
            .assign(&req.team_id, &req.user_id, now_ms)
            .await?;
        let already = prior.members.contains_key(&req.user_id);
        let assigned_at = new.members.get(&req.user_id).copied().unwrap_or(now_ms);

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.team.assigned").expect("hard-coded key parses"),
        )
        .with_param("team", DiagnosticParam::String(new.name.clone()))
        .with_param("user", DiagnosticParam::String(req.user_id.clone()))
        .with_param("at", DiagnosticParam::Timestamp(assigned_at));

        let response = TeamAssignResponse {
            summary,
            team_id: new.team_id,
            user_id: req.user_id,
            already_member: already,
            assigned_at_ms: assigned_at,
        };
        // Stash the prior + new membership maps on the response JSON
        // out-of-band so `change_for` can rebuild the sparse patch
        // without exposing the snapshot to typed REST clients. The
        // dispatcher consumes `output` and `change_for` runs on the
        // same JSON value before any external observer sees it.
        let mut value = serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;
        if !already {
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

impl ReversibleTool for TeamAssignTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let obj = output.as_object()?;
        if obj
            .get("already_member")
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
    use crate::team::store::{InMemoryTeamStore, TeamRow};
    use std::collections::BTreeMap;

    async fn seeded() -> Arc<InMemoryTeamStore> {
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
        store
    }

    #[tokio::test]
    async fn first_assign_emits_assigned_and_records_patch_draft() {
        let store = seeded().await;
        let tool = TeamAssignTool::new(store);
        let input = serde_json::json!({"team_id": "t-1", "user_id": "u-1"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: TeamAssignResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.team.assigned");
        assert!(!resp.already_member);

        let draft = tool.change_for(&input, &out).expect("draft present");
        let after: TeamPatch = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert!(after
            .members
            .as_ref()
            .map(|m| m.contains_key("u-1"))
            .unwrap_or(false));
    }

    #[tokio::test]
    async fn second_assign_is_idempotent_and_skips_draft() {
        let store = seeded().await;
        let tool = TeamAssignTool::new(store);
        let input = serde_json::json!({"team_id": "t-1", "user_id": "u-1"});
        let _ = tool.invoke(input.clone()).await.unwrap();
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: TeamAssignResponse = serde_json::from_value(out.clone()).unwrap();
        assert!(resp.already_member);
        assert!(tool.change_for(&input, &out).is_none());
    }
}
