//! `rubix.user.disable` — tool dispatch.
//!
//! Marks an existing user as disabled. The verb is idempotent — a
//! second call against an already-disabled user returns the
//! `rubix.user.already_disabled` diagnostic and produces *no*
//! `ChangeDraft` (so undo cannot accidentally unwind a no-op).
//!
//! Snapshot shape: `Op::Update`, `before` = the prior [`UserRow`]
//! (with `disabled_at_ms = None`), `after` = the new row with
//! `disabled_at_ms = Some(epoch_ms)`. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::user::disable::{UserDisableRequest, UserDisableResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::undo::dispatch::ReversibleTool;
use crate::user::store::{UserAdminStore, UserRow, USER_KIND};

/// Concrete [`Tool`] for `rubix.user.disable`.
pub struct UserDisableTool {
    store: Arc<dyn UserAdminStore>,
}

impl UserDisableTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn UserAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for UserDisableTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.user.disable".to_owned(),
            description: rubix_spi::dto::user::disable::DESCRIPTOR.purpose.to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "user_id": { "type": ["string", "null"] },
                    "email": { "type": ["string", "null"] }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: UserDisableRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("UserDisableRequest: {e}"),
            })?;

        let target = resolve_target(&*self.store, &req).await?;
        let now_ms = now_epoch_ms();
        let (prior, new) = self.store.disable(&target.user_id, now_ms).await?;
        let was_already = prior.disabled_at_ms.is_some();

        let key = if was_already {
            "rubix.user.already_disabled"
        } else {
            "rubix.user.disabled"
        };
        let summary = Diagnostic::new(MessageKey::parse(key).expect("hard-coded key parses"))
            .with_param("email", DiagnosticParam::String(new.email.clone()))
            .with_param(
                "at",
                DiagnosticParam::Timestamp(new.disabled_at_ms.unwrap_or(now_ms)),
            );

        let response = UserDisableResponse {
            summary,
            user_id: new.user_id,
            email: new.email,
            role: new.role,
            was_already_disabled: was_already,
            disabled_at_ms: new.disabled_at_ms.unwrap_or(now_ms),
            // Echo prefs from the live row so `change_for`
            // reconstructs the full prior snapshot. `prior` and
            // `new` share `prefs_json` (disable doesn't touch
            // prefs), so either side is correct — pick `prior`
            // for clarity that this is the pre-mutation value.
            prefs_json: prior.prefs_json.clone(),
            // Echo tenant assignment for the same reason. disable
            // doesn't touch the tenant, so `prior` carries the
            // post-mutation value too. Required for byte-exact
            // snapshot reconstruction in `change_for`.
            tenant_id: prior.tenant_id.clone(),
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for UserDisableTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: UserDisableResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_already_disabled {
            // No state change — recording would let undo silently
            // re-enable a user the caller did not actually flip.
            return None;
        }
        let before = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.role.clone(),
            disabled_at_ms: None,
            prefs_json: resp.prefs_json.clone(),
            tenant_id: resp.tenant_id.clone(),
        };
        let after = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.role.clone(),
            disabled_at_ms: Some(resp.disabled_at_ms),
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

async fn resolve_target(store: &dyn UserAdminStore, req: &UserDisableRequest) -> Result<UserRow> {
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
        message: "UserDisableRequest requires user_id or email".to_owned(),
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

    async fn seeded() -> (Arc<InMemoryUserStore>, UserRow) {
        let store = Arc::new(InMemoryUserStore::new());
        let row = UserRow {
            user_id: "u-1".into(),
            email: "ada@x".into(),
            role: "admin".into(),
            disabled_at_ms: None,
            prefs_json: None,
            tenant_id: None,
        };
        store.create(row.clone()).await.unwrap();
        (store, row)
    }

    #[tokio::test]
    async fn disable_first_time_emits_disabled_diagnostic() {
        let (store, _) = seeded().await;
        let tool = UserDisableTool::new(store);
        let out = tool
            .invoke(serde_json::json!({"email": "ada@x"}))
            .await
            .unwrap();
        let resp: UserDisableResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.disabled");
        assert!(!resp.was_already_disabled);
    }

    #[tokio::test]
    async fn disable_again_emits_already_disabled_and_skips_draft() {
        let (store, _) = seeded().await;
        let tool = UserDisableTool::new(store);
        let input = serde_json::json!({"email": "ada@x"});
        let _ = tool.invoke(input.clone()).await.unwrap();
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: UserDisableResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.already_disabled");
        assert!(resp.was_already_disabled);
        assert!(
            tool.change_for(&input, &out).is_none(),
            "no-op disable must not record a Change",
        );
    }

    #[tokio::test]
    async fn change_for_records_update_with_before_after_snapshots() {
        let (store, _) = seeded().await;
        let tool = UserDisableTool::new(store);
        let input = serde_json::json!({"user_id": "u-1"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft present");
        assert!(matches!(draft.op, Op::Update));
        let before: UserRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: UserRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert!(before.disabled_at_ms.is_none());
        assert!(after.disabled_at_ms.is_some());
    }
}
