//! `rubix.team.update` — tool dispatch.
//!
//! Renames a team and/or updates its description. Mirrors the
//! shape of [`crate::tenant::update`] except the `ChangeDraft`
//! emits a [`crate::team::store::TeamPatch`] (only the fields
//! that flipped) rather than a full snapshot — matching the
//! patch-shape contract of [`crate::team::store::TeamReversible`].
//!
//! Uniqueness on rename: the verb walks `store.get` for each row
//! that might collide (via a `list`-free path — see implementation
//! note inline). For the in-memory store today the check is the
//! same shape as `tenant.update`: walk + filter-self + match.
//! A PG-backed `TeamAdminStore` will enforce the same invariant
//! through a unique index on `name`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::team::update::{TeamUpdateRequest, TeamUpdateResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::team::store::{TeamAdminStore, TeamPatch, TEAM_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.team.update`.
pub struct TeamUpdateTool {
    store: Arc<dyn TeamAdminStore>,
}

impl TeamUpdateTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TeamAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TeamUpdateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.team.update".to_owned(),
            description: rubix_spi::dto::team::update::DESCRIPTOR.purpose.to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "team_id":     { "type": "string", "minLength": 1 },
                    "name":        { "type": ["string", "null"] },
                    "description": { "type": ["string", "null"] }
                },
                "required": ["team_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: TeamUpdateRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("TeamUpdateRequest: {e}"),
        })?;

        if req.team_id.is_empty() || req.team_id.trim() != req.team_id {
            return Err(Error::Invalid {
                message: "TeamUpdateRequest.team_id must be non-empty and trimmed".to_owned(),
            });
        }
        if req.name.is_none() && req.description.is_none() {
            return Err(Error::Invalid {
                message: "TeamUpdateRequest requires at least one of name / description".to_owned(),
            });
        }
        if let Some(name) = req.name.as_deref() {
            if name.is_empty() || name.trim() != name {
                return Err(Error::Invalid {
                    message: "TeamUpdateRequest.name must be non-empty and trimmed".to_owned(),
                });
            }
        }

        let prior = self
            .store
            .get(&req.team_id)
            .await?
            .ok_or_else(|| Error::NotFound {
                what: format!("team:{}", req.team_id),
            })?;

        let new_name = req.name.clone().unwrap_or_else(|| prior.name.clone());
        let new_description = match req.description.clone() {
            Some(d) => Some(d),
            None => prior.description.clone(),
        };

        let was_unchanged = new_name == prior.name && new_description == prior.description;
        let updated_at_ms = now_epoch_ms();

        if !was_unchanged && new_name != prior.name {
            // Uniqueness-excluding-self check on rename. The
            // `TeamAdminStore` trait does not expose `list` today
            // (membership management does not need it), but the
            // in-memory store implements it through `get`-by-id;
            // for PG, the unique index does the work and this
            // pre-check becomes a fast path. We deliberately scan
            // through the store API rather than reaching into the
            // concrete type so the verb stays trait-bound.
            //
            // Implementation note: the trait *does* expose `get`,
            // not `list`. We work around by attempting `create` on
            // a probe row? No — that would mutate. The clean shape
            // is to add a `list` method to the trait. Adding it
            // here in the verb file is wrong layering; the slice
            // adds `list` to the trait below.
            let rows = self.store.list().await?;
            let collision = rows
                .into_iter()
                .any(|r| r.team_id != prior.team_id && r.name == new_name);
            if collision {
                return Err(Error::Conflict {
                    message: format!("team with name {new_name} already exists"),
                });
            }
        }

        if !was_unchanged {
            let mut next = prior.clone();
            next.name = new_name.clone();
            next.description = new_description.clone();
            self.store.put(next).await?;
        }

        let key = if was_unchanged {
            "rubix.team.unchanged"
        } else {
            "rubix.team.updated"
        };
        let mut diag = Diagnostic::new(MessageKey::parse(key).expect("hard-coded key parses"))
            .with_param("team", DiagnosticParam::String(prior.team_id.clone()))
            .with_param("name", DiagnosticParam::String(new_name.clone()))
            .with_param("at", DiagnosticParam::Timestamp(updated_at_ms));
        if !was_unchanged {
            diag = diag.with_param("prior_name", DiagnosticParam::String(prior.name.clone()));
        }

        let response = TeamUpdateResponse {
            summary: diag,
            team_id: prior.team_id,
            prior_name: prior.name,
            new_name,
            prior_description: prior.description,
            new_description,
            was_unchanged,
            updated_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for TeamUpdateTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: TeamUpdateResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_unchanged {
            return None;
        }
        // Patch shape: only the fields the verb actually flipped
        // land in `before`/`after`. Membership is untouched, so
        // `patch.members` stays `None` — concurrent assign /
        // unassign edits on disjoint fields will not clobber on
        // undo (see TeamReversible::apply_inverse merge logic).
        let name_flipped = resp.prior_name != resp.new_name;
        let desc_flipped = resp.prior_description != resp.new_description;
        let before = TeamPatch {
            name: name_flipped.then(|| resp.prior_name.clone()),
            description: desc_flipped.then(|| resp.prior_description.clone()),
            members: None,
        };
        let after = TeamPatch {
            name: name_flipped.then(|| resp.new_name.clone()),
            description: desc_flipped.then(|| resp.new_description.clone()),
            members: None,
        };
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: TEAM_KIND.into(),
                id: Some(resp.team_id),
                owner: None,
                tenant: None,
            },
            op: Op::Update,
            before: Some(serde_json::to_value(&before).ok()?),
            after: Some(serde_json::to_value(&after).ok()?),
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
    use crate::team::store::{InMemoryTeamStore, TeamRow};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn row(id: &str, name: &str, desc: Option<&str>) -> TeamRow {
        TeamRow {
            team_id: id.into(),
            name: name.into(),
            description: desc.map(str::to_owned),
            members: BTreeMap::new(),
        }
    }

    async fn seeded() -> Arc<InMemoryTeamStore> {
        let store = Arc::new(InMemoryTeamStore::new());
        store.create(row("t-1", "Ops", None)).await.unwrap();
        store
            .create(row("t-2", "Eng", Some("Engineers")))
            .await
            .unwrap();
        store
    }

    #[tokio::test]
    async fn rename_changes_name_and_emits_updated() {
        let store = seeded().await;
        let tool = TeamUpdateTool::new(store.clone());
        let out = tool
            .invoke(json!({"team_id": "t-1", "name": "Operations"}))
            .await
            .unwrap();
        let resp: TeamUpdateResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.team.updated");
        assert!(!resp.was_unchanged);
        assert_eq!(resp.prior_name, "Ops");
        assert_eq!(resp.new_name, "Operations");
        let row = store.get("t-1").await.unwrap().unwrap();
        assert_eq!(row.name, "Operations");
    }

    #[tokio::test]
    async fn re_describe_changes_description_only() {
        let store = seeded().await;
        let tool = TeamUpdateTool::new(store.clone());
        let out = tool
            .invoke(json!({"team_id": "t-1", "description": "On-call rotation"}))
            .await
            .unwrap();
        let resp: TeamUpdateResponse = serde_json::from_value(out).unwrap();
        assert!(!resp.was_unchanged);
        assert_eq!(resp.new_name, "Ops");
        assert_eq!(resp.new_description.as_deref(), Some("On-call rotation"));
    }

    #[tokio::test]
    async fn clear_description_via_empty_string() {
        let store = seeded().await;
        let tool = TeamUpdateTool::new(store.clone());
        let out = tool
            .invoke(json!({"team_id": "t-2", "description": ""}))
            .await
            .unwrap();
        let resp: TeamUpdateResponse = serde_json::from_value(out).unwrap();
        assert!(!resp.was_unchanged);
        assert_eq!(resp.prior_description.as_deref(), Some("Engineers"));
        assert_eq!(resp.new_description.as_deref(), Some(""));
    }

    #[tokio::test]
    async fn rename_to_existing_name_is_rejected_as_conflict() {
        let store = seeded().await;
        let tool = TeamUpdateTool::new(store);
        let err = tool
            .invoke(json!({"team_id": "t-1", "name": "Eng"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Conflict { .. }));
    }

    #[tokio::test]
    async fn unchanged_path_skips_draft() {
        let store = seeded().await;
        let tool = TeamUpdateTool::new(store);
        let input = json!({"team_id": "t-1", "name": "Ops"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: TeamUpdateResponse = serde_json::from_value(out.clone()).unwrap();
        assert!(resp.was_unchanged);
        assert_eq!(resp.summary.code.as_str(), "rubix.team.unchanged");
        assert!(tool.change_for(&input, &out).is_none());
    }

    #[tokio::test]
    async fn missing_team_returns_not_found() {
        let store = Arc::new(InMemoryTeamStore::new());
        let tool = TeamUpdateTool::new(store);
        let err = tool
            .invoke(json!({"team_id": "t-ghost", "name": "Anything"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn no_fields_supplied_is_rejected() {
        let store = seeded().await;
        let tool = TeamUpdateTool::new(store);
        let err = tool.invoke(json!({"team_id": "t-1"})).await.unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn empty_name_is_rejected() {
        let store = seeded().await;
        let tool = TeamUpdateTool::new(store);
        let err = tool
            .invoke(json!({"team_id": "t-1", "name": ""}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn change_for_emits_patch_with_only_flipped_fields() {
        let store = seeded().await;
        let tool = TeamUpdateTool::new(store);
        // Rename only — description patch slot must stay None.
        let input = json!({"team_id": "t-1", "name": "Operations"});
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft");
        assert!(matches!(draft.op, Op::Update));
        let before: TeamPatch = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: TeamPatch = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(before.name.as_deref(), Some("Ops"));
        assert_eq!(after.name.as_deref(), Some("Operations"));
        assert!(before.description.is_none(), "untouched fields stay None");
        assert!(after.description.is_none());
        assert!(before.members.is_none());
        assert!(after.members.is_none());
    }

    #[tokio::test]
    async fn rename_preserves_membership_via_put() {
        let store = Arc::new(InMemoryTeamStore::new());
        store.create(row("t-1", "Ops", None)).await.unwrap();
        store.assign("t-1", "u-1", 100).await.unwrap();
        let tool = TeamUpdateTool::new(store.clone());
        let _ = tool
            .invoke(json!({"team_id": "t-1", "name": "Operations"}))
            .await
            .unwrap();
        let row = store.get("t-1").await.unwrap().unwrap();
        assert_eq!(row.name, "Operations");
        assert_eq!(
            row.members.get("u-1"),
            Some(&100),
            "rename must preserve membership map",
        );
    }
}
