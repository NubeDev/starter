//! `rubix.user.delete` \u{2014} tool dispatch.
//!
//! Hard-deletes an existing user via the shared
//! [`UserAdminStore`]. Refuses with `rubix.user.in_teams` when
//! the user is a member of any team \u{2014} the operator must
//! `rubix.team.member.unassign` from every team first.
//!
//! See the DTO module doc for the cascade decision rationale
//! (mirrors `rubix.tenant.delete`).
//!
//! Snapshot shape: `Op::Delete`, `before` = the full prior
//! [`UserRow`] (so undo can re-create the row including
//! `disabled_at_ms`, `prefs_json`, `tenant_id`),
//! `after = None`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::user::delete::{UserDeleteRequest, UserDeleteResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::team::store::TeamAdminStore;
use crate::undo::dispatch::ReversibleTool;
use crate::user::store::{UserAdminStore, UserRow, USER_KIND};

/// Concrete [`Tool`] for `rubix.user.delete`.
pub struct UserDeleteTool {
    users: Arc<dyn UserAdminStore>,
    teams: Arc<dyn TeamAdminStore>,
}

impl UserDeleteTool {
    /// Wrap the shared stores. The team store is used to enforce
    /// the refuse-if-member-of-any-team cascade decision before
    /// the user store mutates (mirrors
    /// [`crate::tenant::delete::TenantDeleteTool`]).
    pub fn new(users: Arc<dyn UserAdminStore>, teams: Arc<dyn TeamAdminStore>) -> Self {
        Self { users, teams }
    }
}

#[async_trait]
impl Tool for UserDeleteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.user.delete".to_owned(),
            description: rubix_spi::dto::user::delete::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "user_id": { "type": ["string", "null"] },
                    "email":   { "type": ["string", "null"] }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: UserDeleteRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("UserDeleteRequest: {e}"),
            })?;

        // Resolve the row before any cascade check so we can
        // return a structured NotFound and so the snapshot we
        // record on success has every identity-bearing field.
        let prior = resolve_target(&*self.users, &req).await?;

        // Cascade check: refuse if the user is a member of any
        // team. We surface the names so the operator can run
        // `rubix.team.member.unassign` explicitly. Cap the
        // echoed list at 10 to keep the diagnostic readable;
        // the count is authoritative.
        let team_names: Vec<String> = self
            .teams
            .list()
            .await?
            .into_iter()
            .filter(|t| t.members.contains_key(&prior.user_id))
            .map(|t| t.name)
            .collect();
        if !team_names.is_empty() {
            let count = team_names.len();
            let preview: Vec<String> = team_names.iter().take(10).cloned().collect();
            let diag = Diagnostic::new(
                MessageKey::parse("rubix.user.in_teams").expect("hard-coded key parses"),
            )
            .with_param("user", DiagnosticParam::String(prior.user_id.clone()))
            .with_param("email", DiagnosticParam::String(prior.email.clone()))
            .with_param("count", DiagnosticParam::I64(count as i64))
            .with_param("teams", DiagnosticParam::String(preview.join(", ")));
            return Err(Error::Conflict {
                message: serde_json::to_string(&diag).unwrap_or_else(|_| {
                    format!(
                        "user {} is a member of {count} team(s); unassign first",
                        prior.email,
                    )
                }),
            });
        }

        self.users.delete(&prior.user_id).await?;
        let deleted_at_ms = now_epoch_ms();
        let summary = Diagnostic::new(
            MessageKey::parse("rubix.user.deleted").expect("hard-coded key parses"),
        )
        .with_param("email", DiagnosticParam::String(prior.email.clone()))
        .with_param("user", DiagnosticParam::String(prior.user_id.clone()))
        .with_param("at", DiagnosticParam::Timestamp(deleted_at_ms));

        let response = UserDeleteResponse {
            summary,
            user_id: prior.user_id,
            email: prior.email,
            role: prior.role,
            disabled_at_ms: prior.disabled_at_ms,
            prefs_json: prior.prefs_json,
            tenant_id: prior.tenant_id,
            deleted_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for UserDeleteTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: UserDeleteResponse = serde_json::from_value(output.clone()).ok()?;
        // Full-row `before` snapshot reconstructed from the
        // response \u{2014} `change_for` does not re-read the
        // store because the row no longer exists after the
        // delete succeeded. The DTO carries every
        // identity-bearing field for exactly this reason
        // (\u{00A7}3.1 echo rule).
        let before = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email,
            role: resp.role,
            disabled_at_ms: resp.disabled_at_ms,
            prefs_json: resp.prefs_json,
            tenant_id: resp.tenant_id,
        };
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: USER_KIND.into(),
                id: Some(resp.user_id),
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

async fn resolve_target(store: &dyn UserAdminStore, req: &UserDeleteRequest) -> Result<UserRow> {
    if let Some(id) = &req.user_id {
        return store.get(id).await?.ok_or_else(|| Error::NotFound {
            what: format!("user:{id}"),
        });
    }
    if let Some(email) = &req.email {
        return store
            .find_by_email(email)
            .await?
            .ok_or_else(|| Error::NotFound {
                what: format!("user(email):{email}"),
            });
    }
    Err(Error::Invalid {
        message: "UserDeleteRequest requires user_id or email".to_owned(),
    })
}

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::team::store::{InMemoryTeamStore, TeamRow};
    use crate::user::store::InMemoryUserStore;
    use serde_json::json;

    fn user(id: &str, email: &str) -> UserRow {
        UserRow {
            user_id: id.into(),
            email: email.into(),
            role: "reader".into(),
            disabled_at_ms: None,
            prefs_json: None,
            tenant_id: None,
        }
    }

    fn team(id: &str, name: &str, members: &[(&str, i64)]) -> TeamRow {
        let mut m = BTreeMap::new();
        for (uid, ts) in members {
            m.insert((*uid).to_owned(), *ts);
        }
        TeamRow {
            team_id: id.into(),
            name: name.into(),
            description: None,
            members: m,
        }
    }

    async fn seeded() -> (Arc<InMemoryUserStore>, Arc<InMemoryTeamStore>) {
        let users = Arc::new(InMemoryUserStore::new());
        users.create(user("u-1", "ada@x")).await.unwrap();
        let teams = Arc::new(InMemoryTeamStore::new());
        (users, teams)
    }

    #[tokio::test]
    async fn delete_unassigned_user_succeeds() {
        let (users, teams) = seeded().await;
        let tool = UserDeleteTool::new(users.clone(), teams);
        let out = tool.invoke(json!({"email": "ada@x"})).await.unwrap();
        let resp: UserDeleteResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.deleted");
        assert_eq!(resp.user_id, "u-1");
        assert_eq!(resp.email, "ada@x");
        assert!(users.get("u-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_refuses_when_user_in_any_team() {
        let (users, teams) = seeded().await;
        teams
            .create(team("t-ops", "Ops", &[("u-1", 100)]))
            .await
            .unwrap();
        teams
            .create(team("t-sec", "Security", &[("u-1", 200), ("u-2", 300)]))
            .await
            .unwrap();
        let tool = UserDeleteTool::new(users.clone(), teams);
        let err = tool.invoke(json!({"email": "ada@x"})).await.unwrap_err();
        match err {
            Error::Conflict { message } => {
                assert!(
                    message.contains("rubix.user.in_teams"),
                    "diagnostic code present: {message}"
                );
                // Both team names should be named in the
                // params blob.
                assert!(message.contains("Ops"));
                assert!(message.contains("Security"));
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
        // Row must still be present \u{2014} refuse blocked the
        // delete.
        assert!(users.get("u-1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn delete_missing_user_returns_not_found() {
        let (_users, teams) = seeded().await;
        let empty = Arc::new(InMemoryUserStore::new());
        let tool = UserDeleteTool::new(empty, teams);
        let err = tool
            .invoke(json!({"user_id": "missing"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }), "got {err:?}");
    }

    #[tokio::test]
    async fn change_for_echoes_full_prior_snapshot() {
        let (users, teams) = seeded().await;
        // Mutate the seeded row so the snapshot has every field
        // populated \u{2014} pins the \u{00A7}3.1 echo rule for
        // this verb.
        users
            .set_role("u-1", "admin")
            .await
            .unwrap();
        users
            .set_prefs("u-1", json!({"locale": "en-US"}))
            .await
            .unwrap();
        users.disable("u-1", 999).await.unwrap();

        let tool = UserDeleteTool::new(users, teams);
        let out = tool.invoke(json!({"user_id": "u-1"})).await.unwrap();
        let draft = tool.change_for(&json!({}), &out).expect("draft");
        assert_eq!(draft.op, Op::Delete);
        let before: UserRow =
            serde_json::from_value(draft.before.expect("before present")).unwrap();
        assert_eq!(before.user_id, "u-1");
        assert_eq!(before.email, "ada@x");
        assert_eq!(before.role, "admin");
        assert_eq!(before.disabled_at_ms, Some(999));
        assert_eq!(before.prefs_json, Some(json!({"locale": "en-US"})));
        assert!(draft.after.is_none());
    }
}
