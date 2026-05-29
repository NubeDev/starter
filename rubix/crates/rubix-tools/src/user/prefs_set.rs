//! `rubix.user.prefs.set` — tool dispatch.
//!
//! Replaces the prefs blob on an existing user row. Idempotent — a
//! second call with a blob equal to the stored value returns the
//! `rubix.user.prefs.unchanged` diagnostic and produces *no*
//! `ChangeDraft` (so undo cannot accidentally rewrite a blob that
//! never changed).
//!
//! Snapshot shape: `Op::Update`, full `UserRow` on both sides.
//! Every identity-bearing field rides on the response so the
//! `change_for` adapter reconstructs the snapshot byte-exact
//! without a follow-up store read — same posture as `role_set.rs`
//! and the dashboard rename fix (proposal §3.1).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::user::prefs_set::{UserPrefsSetRequest, UserPrefsSetResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::undo::dispatch::ReversibleTool;
use crate::user::store::{UserAdminStore, UserRow, USER_KIND};

/// Concrete [`Tool`] for `rubix.user.prefs.set`.
pub struct UserPrefsSetTool {
    store: Arc<dyn UserAdminStore>,
}

impl UserPrefsSetTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn UserAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for UserPrefsSetTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.user.prefs.set".to_owned(),
            description: rubix_spi::dto::user::prefs_set::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "user_id": { "type": ["string", "null"] },
                    "email":   { "type": ["string", "null"] },
                    "prefs":   {}
                },
                "required": ["prefs"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: UserPrefsSetRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("UserPrefsSetRequest: {e}"),
            })?;

        let target = resolve_target(&*self.store, &req).await?;
        let (prior, new) = self.store.set_prefs(&target.user_id, req.prefs).await?;
        let was_unchanged = prior.prefs_json == new.prefs_json;

        let key = if was_unchanged {
            "rubix.user.prefs.unchanged"
        } else {
            "rubix.user.prefs.set"
        };
        let now_ms = now_epoch_ms();
        let diag = Diagnostic::new(MessageKey::parse(key).expect("hard-coded key parses"))
            .with_param("email", DiagnosticParam::String(new.email.clone()))
            .with_param("at", DiagnosticParam::Timestamp(now_ms));

        let response = UserPrefsSetResponse {
            summary: diag,
            user_id: new.user_id.clone(),
            email: new.email.clone(),
            prior_prefs: prior.prefs_json,
            new_prefs: new.prefs_json.unwrap_or(Value::Null),
            was_unchanged,
            // Echo the row's other identity-bearing fields so the
            // snapshot reconstructs byte-exact (see module doc).
            role: new.role,
            disabled_at_ms: new.disabled_at_ms,
            tenant_id: new.tenant_id,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for UserPrefsSetTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: UserPrefsSetResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_unchanged {
            // No state change — recording would let undo silently
            // rewrite a prefs blob the caller did not actually
            // flip. Mirrors the `was_unchanged` short-circuit in
            // `UserRoleSetTool::change_for`.
            return None;
        }
        // Full `UserRow` snapshot on both sides. prefs_set doesn't
        // touch `role` or `disabled_at_ms`, so both halves share
        // them. `new_prefs` is the after-side; `prior_prefs` is
        // the before-side and may be `None` (no prefs row before).
        // The after-side stores `Some(new_prefs)` — even when
        // `new_prefs` is `Value::Null` the row carries
        // `Some(Null)`, matching the store contract.
        let before = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.role.clone(),
            disabled_at_ms: resp.disabled_at_ms,
            prefs_json: resp.prior_prefs.clone(),
            tenant_id: resp.tenant_id.clone(),
        };
        let after = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.role.clone(),
            disabled_at_ms: resp.disabled_at_ms,
            prefs_json: Some(resp.new_prefs.clone()),
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

async fn resolve_target(store: &dyn UserAdminStore, req: &UserPrefsSetRequest) -> Result<UserRow> {
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
        message: "UserPrefsSetRequest requires user_id or email".to_owned(),
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
    use serde_json::json;
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
    async fn set_prefs_on_blank_row_changes_none_to_some() {
        let store = seeded().await;
        let tool = UserPrefsSetTool::new(store);
        let out = tool
            .invoke(json!({"email": "ada@x", "prefs": {"locale": "es-ES"}}))
            .await
            .unwrap();
        let resp: UserPrefsSetResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.prefs.set");
        assert!(resp.prior_prefs.is_none());
        assert_eq!(resp.new_prefs, json!({"locale": "es-ES"}));
        assert!(!resp.was_unchanged);
    }

    #[tokio::test]
    async fn set_same_prefs_is_noop_and_skips_draft() {
        let store = seeded().await;
        let tool = UserPrefsSetTool::new(store);
        let input = json!({"user_id": "u-1", "prefs": {"locale": "en-US"}});
        let _ = tool.invoke(input.clone()).await.unwrap();
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: UserPrefsSetResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.prefs.unchanged");
        assert!(resp.was_unchanged);
        assert!(
            tool.change_for(&input, &out).is_none(),
            "no-op prefs-set must not record a Change",
        );
    }

    #[tokio::test]
    async fn change_for_snapshot_carries_full_row_with_prior_prefs() {
        let store = seeded().await;
        let tool = UserPrefsSetTool::new(store);
        // First call: set initial prefs.
        let _ = tool
            .invoke(json!({"user_id": "u-1", "prefs": {"a": 1}}))
            .await
            .unwrap();
        // Second call: change prefs. The snapshot's `before` must
        // carry `{a: 1}` (the prior live value), not `None` or the
        // new value — otherwise undo silently clears the prior
        // prefs.
        let input = json!({"user_id": "u-1", "prefs": {"a": 2}});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft present");
        assert!(matches!(draft.op, Op::Update));
        let before: UserRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: UserRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(before.prefs_json, Some(json!({"a": 1})));
        assert_eq!(after.prefs_json, Some(json!({"a": 2})));
        // The other identity-bearing fields round-trip — pins the
        // contract that the snapshot is the full row, not a delta.
        assert_eq!(before.role, after.role);
        assert_eq!(before.email, after.email);
        assert_eq!(before.disabled_at_ms, after.disabled_at_ms);
    }

    #[tokio::test]
    async fn snapshot_byte_exact_preserves_role_and_disabled_state() {
        // Load-bearing for the §3.1-bug-class avoidance: a prefs
        // change on a disabled, admin-role user must not surface
        // a snapshot that silently re-enables them or downgrades
        // their role on undo.
        let store = Arc::new(InMemoryUserStore::new());
        store
            .create(UserRow {
                user_id: "u-1".into(),
                email: "bob@x".into(),
                role: "admin".into(),
                disabled_at_ms: Some(1_700_000_000_000),
                prefs_json: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        let tool = UserPrefsSetTool::new(store);
        let input = json!({"user_id": "u-1", "prefs": {"k": "v"}});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft");
        let before: UserRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: UserRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(before.role, "admin");
        assert_eq!(after.role, "admin");
        assert_eq!(before.disabled_at_ms, Some(1_700_000_000_000));
        assert_eq!(after.disabled_at_ms, Some(1_700_000_000_000));
    }

    #[tokio::test]
    async fn missing_target_returns_not_found() {
        let store = seeded().await;
        let tool = UserPrefsSetTool::new(store);
        let err = tool
            .invoke(json!({"email": "missing@x", "prefs": {}}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn null_prefs_is_legal_and_stored_as_some_null() {
        let store = seeded().await;
        let tool = UserPrefsSetTool::new(store);
        let out = tool
            .invoke(json!({"user_id": "u-1", "prefs": null}))
            .await
            .unwrap();
        let resp: UserPrefsSetResponse = serde_json::from_value(out).unwrap();
        assert!(!resp.was_unchanged, "None → Some(Null) is a state change");
        assert_eq!(resp.new_prefs, Value::Null);
    }
}
