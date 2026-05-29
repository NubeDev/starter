//! `rubix.user.enable` — tool dispatch.
//!
//! Inverse of [`crate::user::disable`]. Clears `disabled_at_ms`
//! back to `None`. Idempotent — a second call against an
//! already-enabled user returns the
//! `rubix.user.already_enabled` diagnostic and produces *no*
//! `ChangeDraft` (so undo cannot accidentally re-disable a user
//! the caller did not actually flip). Mirrors `disable`'s
//! `was_already_disabled` posture.
//!
//! Snapshot shape: `Op::Update`, `before` = the prior [`UserRow`]
//! (with `disabled_at_ms = Some(prior_ts)`), `after` = the new
//! row with `disabled_at_ms = None`. The `prior_disabled_at_ms`
//! field on the response carries the original timestamp so undo
//! restores it byte-exact rather than producing a fresh `now()`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::user::enable::{UserEnableRequest, UserEnableResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::undo::dispatch::ReversibleTool;
use crate::user::store::{UserAdminStore, UserRow, USER_KIND};

/// Concrete [`Tool`] for `rubix.user.enable`.
pub struct UserEnableTool {
    store: Arc<dyn UserAdminStore>,
}

impl UserEnableTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn UserAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for UserEnableTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.user.enable".to_owned(),
            description: rubix_spi::dto::user::enable::DESCRIPTOR.purpose.to_owned(),
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
        let req: UserEnableRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("UserEnableRequest: {e}"),
        })?;

        let target = resolve_target(&*self.store, &req).await?;
        let now_ms = now_epoch_ms();
        let (prior, new) = self.store.enable(&target.user_id).await?;
        let was_already = prior.disabled_at_ms.is_none();

        let key = if was_already {
            "rubix.user.already_enabled"
        } else {
            "rubix.user.enabled"
        };
        let summary = Diagnostic::new(MessageKey::parse(key).expect("hard-coded key parses"))
            .with_param("email", DiagnosticParam::String(new.email.clone()))
            .with_param("at", DiagnosticParam::Timestamp(now_ms));

        let response = UserEnableResponse {
            summary,
            user_id: new.user_id,
            email: new.email,
            role: new.role,
            // Echo from `prior` for clarity: enable doesn't touch
            // prefs/tenant, so prior and new agree; using prior
            // documents "the snapshot value", not a post-mutation
            // side effect.
            prefs_json: prior.prefs_json.clone(),
            tenant_id: prior.tenant_id.clone(),
            was_already_enabled: was_already,
            // Carries the ORIGINAL disabled_at_ms timestamp so
            // `change_for` can reconstruct the `before` snapshot
            // byte-exact. None when was_already_enabled (no draft
            // is recorded in that case anyway).
            prior_disabled_at_ms: prior.disabled_at_ms,
            enabled_at_ms: now_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for UserEnableTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: UserEnableResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_already_enabled {
            // No state change — recording would let undo silently
            // re-disable a user the caller did not actually flip.
            return None;
        }
        let before = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.role.clone(),
            // Byte-exact restore of the original disabled-at
            // timestamp. Required for the snapshot contract — undo
            // restores the prior `disabled_at_ms`, not `Some(now)`.
            disabled_at_ms: resp.prior_disabled_at_ms,
            prefs_json: resp.prefs_json.clone(),
            tenant_id: resp.tenant_id.clone(),
        };
        let after = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.role.clone(),
            disabled_at_ms: None,
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

async fn resolve_target(store: &dyn UserAdminStore, req: &UserEnableRequest) -> Result<UserRow> {
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
        message: "UserEnableRequest requires user_id or email".to_owned(),
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
    use crate::user::store::{InMemoryUserStore, UserReversible};
    use starter_spi::changelog::{Actor, Change, ChangeId, GroupId, Op, Reversible};

    async fn seeded_disabled() -> (Arc<InMemoryUserStore>, UserRow) {
        let store = Arc::new(InMemoryUserStore::new());
        let row = UserRow {
            user_id: "u-1".into(),
            email: "ada@x".into(),
            role: "admin".into(),
            disabled_at_ms: Some(100),
            prefs_json: Some(serde_json::json!({"theme": "dark"})),
            tenant_id: Some("t-acme".into()),
        };
        store.put(row.clone()).await.unwrap();
        (store, row)
    }

    async fn seeded_enabled() -> (Arc<InMemoryUserStore>, UserRow) {
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
    async fn enable_disabled_user_emits_enabled_diagnostic() {
        let (store, _) = seeded_disabled().await;
        let tool = UserEnableTool::new(store.clone());
        let out = tool
            .invoke(serde_json::json!({"email": "ada@x"}))
            .await
            .unwrap();
        let resp: UserEnableResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.enabled");
        assert!(!resp.was_already_enabled);
        assert_eq!(resp.prior_disabled_at_ms, Some(100));
        // Live row reflects the enable.
        assert!(store
            .get("u-1")
            .await
            .unwrap()
            .unwrap()
            .disabled_at_ms
            .is_none());
    }

    #[tokio::test]
    async fn enable_already_enabled_emits_already_and_skips_draft() {
        let (store, _) = seeded_enabled().await;
        let tool = UserEnableTool::new(store);
        let input = serde_json::json!({"email": "ada@x"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: UserEnableResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.already_enabled");
        assert!(resp.was_already_enabled);
        assert_eq!(resp.prior_disabled_at_ms, None);
        assert!(
            tool.change_for(&input, &out).is_none(),
            "no-op enable must not record a Change",
        );
    }

    #[tokio::test]
    async fn second_enable_after_real_one_is_idempotent_and_skips_draft() {
        let (store, _) = seeded_disabled().await;
        let tool = UserEnableTool::new(store);
        let input = serde_json::json!({"email": "ada@x"});
        let _ = tool.invoke(input.clone()).await.unwrap();
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: UserEnableResponse = serde_json::from_value(out.clone()).unwrap();
        assert!(resp.was_already_enabled);
        assert!(tool.change_for(&input, &out).is_none());
    }

    #[tokio::test]
    async fn missing_user_returns_not_found() {
        let store = Arc::new(InMemoryUserStore::new());
        let tool = UserEnableTool::new(store);
        let err = tool
            .invoke(serde_json::json!({"user_id": "nope"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn empty_request_is_rejected() {
        let store = Arc::new(InMemoryUserStore::new());
        let tool = UserEnableTool::new(store);
        let err = tool.invoke(serde_json::json!({})).await.unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn change_for_records_update_with_byte_exact_snapshots() {
        let (store, _) = seeded_disabled().await;
        let tool = UserEnableTool::new(store);
        let input = serde_json::json!({"user_id": "u-1"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft expected");
        assert_eq!(draft.op, Op::Update);
        let before: UserRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: UserRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        // Critical: byte-exact prior timestamp, not now().
        assert_eq!(before.disabled_at_ms, Some(100));
        assert_eq!(after.disabled_at_ms, None);
        // Prefs + tenant echoed through so undo restores them.
        assert_eq!(
            before.prefs_json,
            Some(serde_json::json!({"theme": "dark"}))
        );
        assert_eq!(before.tenant_id, Some("t-acme".into()));
        assert_eq!(after.prefs_json, before.prefs_json);
        assert_eq!(after.tenant_id, before.tenant_id);
    }

    #[tokio::test]
    async fn reversible_round_trip_restores_original_disabled_timestamp() {
        // Locks in the byte-exact prior-timestamp contract end to end.
        let (store, _) = seeded_disabled().await;
        let tool = UserEnableTool::new(store.clone());
        let input = serde_json::json!({"user_id": "u-1"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft expected");

        // Construct the Change the dispatcher would persist + replay
        // through UserReversible::apply_inverse.
        let change = Change {
            id: ChangeId("c-1".into()),
            group_id: GroupId("g-1".into()),
            at: chrono::Utc::now(),
            actor: Actor::System,
            resource: draft.resource.clone(),
            op: draft.op,
            before: draft.before.clone(),
            after: draft.after.clone(),
            resource_version: None,
            correlation: None,
            patch: None,
        };
        let reversible = UserReversible::new(store.clone());
        reversible.apply_inverse(&change).await.unwrap();
        let restored = store.get("u-1").await.unwrap().unwrap();
        assert_eq!(restored.disabled_at_ms, Some(100));
        assert_eq!(
            restored.prefs_json,
            Some(serde_json::json!({"theme": "dark"}))
        );
        assert_eq!(restored.tenant_id, Some("t-acme".into()));
    }
}
