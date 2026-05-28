//! `rubix.user.role.set` — tool dispatch.
//!
//! Changes the role string on an existing user row. Idempotent — a
//! second call with the same role returns the
//! `rubix.user.role.unchanged` diagnostic and produces *no*
//! `ChangeDraft` (so undo cannot accidentally rewrite a row that
//! never changed).
//!
//! Snapshot shape: `Op::Update`, `before` = the full prior
//! [`UserRow`] (with the old role), `after` = the same row with the
//! new role. The `UserReversible::apply_inverse` path replays the
//! whole snapshot — undo of a role change rewinds nothing else.
//!
//! Audit posture: see the DTO module doc for the link to the
//! audit-floor mechanism that pins `user` `Change` rows past undo
//! retention.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::user::role_set::{UserRoleSetRequest, UserRoleSetResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::undo::dispatch::ReversibleTool;
use crate::user::store::{UserAdminStore, UserRow, USER_KIND};

/// Concrete [`Tool`] for `rubix.user.role.set`.
pub struct UserRoleSetTool {
    store: Arc<dyn UserAdminStore>,
}

impl UserRoleSetTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn UserAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for UserRoleSetTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.user.role.set".to_owned(),
            description: rubix_spi::dto::user::role_set::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "user_id": { "type": ["string", "null"] },
                    "email":   { "type": ["string", "null"] },
                    "role":    { "type": "string", "minLength": 1 }
                },
                "required": ["role"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: UserRoleSetRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("UserRoleSetRequest: {e}"),
            })?;

        // Trim-check is mandatory: a role string with leading or
        // trailing whitespace would compare unequal to the stored
        // form (idempotency check) and would silently corrupt the
        // role taxonomy. Reject early.
        if req.role.is_empty() || req.role.trim() != req.role {
            return Err(Error::Invalid {
                message: "UserRoleSetRequest.role must be non-empty and trimmed".to_owned(),
            });
        }

        let target = resolve_target(&*self.store, &req).await?;
        let (prior, new) = self.store.set_role(&target.user_id, &req.role).await?;
        let was_unchanged = prior.role == new.role;

        let key = if was_unchanged {
            "rubix.user.role.unchanged"
        } else {
            "rubix.user.role.set"
        };
        let now_ms = now_epoch_ms();
        let mut diag = Diagnostic::new(MessageKey::parse(key).expect("hard-coded key parses"))
            .with_param("email", DiagnosticParam::String(new.email.clone()))
            .with_param("new", DiagnosticParam::String(new.role.clone()))
            .with_param("at", DiagnosticParam::Timestamp(now_ms));
        if !was_unchanged {
            diag = diag.with_param("prior", DiagnosticParam::String(prior.role.clone()));
        }

        let response = UserRoleSetResponse {
            summary: diag,
            user_id: new.user_id,
            email: new.email,
            prior_role: prior.role,
            new_role: new.role,
            was_unchanged,
            // Echo the row's other identity-bearing fields so
            // `change_for` reconstructs the full snapshot byte-
            // exact (not just the role flip). `prior` and `new`
            // share these — role_set doesn't touch them — pick
            // `prior` for clarity. See the dashboard rename fix
            // (proposal §3.1) for the prior bug class.
            disabled_at_ms: prior.disabled_at_ms,
            prefs_json: prior.prefs_json,
            tenant_id: prior.tenant_id,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for UserRoleSetTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: UserRoleSetResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_unchanged {
            // No state change — recording would let undo silently
            // rewrite a role that the caller did not actually flip.
            // Mirrors the `was_already_disabled` short-circuit in
            // `UserDisableTool::change_for`.
            return None;
        }
        // Full `UserRow` snapshot reconstructed from the response.
        // Every identity-bearing field rides on the response so
        // undo of a role change does not collateral-damage
        // unrelated fields (the bug class from proposal §3.1).
        // role_set doesn't touch `disabled_at_ms` or `prefs_json`,
        // so both sides share them.
        let before = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.prior_role.clone(),
            disabled_at_ms: resp.disabled_at_ms,
            prefs_json: resp.prefs_json.clone(),
            tenant_id: resp.tenant_id.clone(),
        };
        let after = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.new_role.clone(),
            disabled_at_ms: resp.disabled_at_ms,
            prefs_json: resp.prefs_json.clone(),
            tenant_id: resp.tenant_id.clone(),
        };
        Some(ChangeDraft::update(
            ResourceRef {
                kind: USER_KIND.into(),
                id: Some(resp.user_id),
                owner: None,
                tenant: None,
            },
            serde_json::to_value(&before).ok()?,
            serde_json::to_value(&after).ok()?,
        ))
    }
}

async fn resolve_target(store: &dyn UserAdminStore, req: &UserRoleSetRequest) -> Result<UserRow> {
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
        message: "UserRoleSetRequest requires user_id or email".to_owned(),
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
    use super::*;
    use crate::user::store::InMemoryUserStore;
    use starter_spi::changelog::Op;

    async fn seeded() -> Arc<InMemoryUserStore> {
        let store = Arc::new(InMemoryUserStore::new());
        let row = UserRow {
            user_id: "u-1".into(),
            email: "ada@x".into(),
            role: "reader".into(),
            disabled_at_ms: None,
            prefs_json: None,
            tenant_id: None,
        };
        store.create(row).await.unwrap();
        store
    }

    #[tokio::test]
    async fn set_role_changes_prior_to_new() {
        let store = seeded().await;
        let tool = UserRoleSetTool::new(store);
        let out = tool
            .invoke(serde_json::json!({"email": "ada@x", "role": "admin"}))
            .await
            .unwrap();
        let resp: UserRoleSetResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.role.set");
        assert_eq!(resp.prior_role, "reader");
        assert_eq!(resp.new_role, "admin");
        assert!(!resp.was_unchanged);
    }

    #[tokio::test]
    async fn set_same_role_is_noop_and_skips_draft() {
        let store = seeded().await;
        let tool = UserRoleSetTool::new(store);
        let input = serde_json::json!({"email": "ada@x", "role": "reader"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: UserRoleSetResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.role.unchanged");
        assert!(resp.was_unchanged);
        assert!(
            tool.change_for(&input, &out).is_none(),
            "no-op role-set must not record a Change",
        );
    }

    #[tokio::test]
    async fn change_for_records_update_with_before_after_snapshots() {
        let store = seeded().await;
        let tool = UserRoleSetTool::new(store);
        let input = serde_json::json!({"user_id": "u-1", "role": "admin"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft present");
        assert!(matches!(draft.op, Op::Update));
        let before: UserRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: UserRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(before.role, "reader");
        assert_eq!(after.role, "admin");
        // Other fields round-trip — pins the contract that the
        // snapshot is the full row, not a delta.
        assert_eq!(before.user_id, after.user_id);
        assert_eq!(before.email, after.email);
        assert_eq!(before.disabled_at_ms, after.disabled_at_ms);
    }

    #[tokio::test]
    async fn empty_role_is_rejected() {
        let store = seeded().await;
        let tool = UserRoleSetTool::new(store);
        let err = tool
            .invoke(serde_json::json!({"user_id": "u-1", "role": ""}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn untrimmed_role_is_rejected() {
        let store = seeded().await;
        let tool = UserRoleSetTool::new(store);
        let err = tool
            .invoke(serde_json::json!({"user_id": "u-1", "role": " admin "}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn missing_target_returns_not_found() {
        let store = seeded().await;
        let tool = UserRoleSetTool::new(store);
        let err = tool
            .invoke(serde_json::json!({"email": "missing@x", "role": "admin"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }
}
