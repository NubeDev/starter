//! `rubix.user.tenant.assign` — tool dispatch.
//!
//! Assigns (or unassigns) a tenant on an existing user row.
//! Idempotent — a second call with the same `tenant_id` returns
//! the `rubix.user.tenant.unchanged` diagnostic and produces *no*
//! `ChangeDraft` (so undo cannot accidentally rewrite an
//! assignment that was never changed).
//!
//! Snapshot shape: `Op::Update`, full `UserRow` on both sides.
//! Every identity-bearing field rides on the response so the
//! `change_for` adapter reconstructs the snapshot byte-exact
//! without a follow-up store read — same posture as `role_set.rs`
//! and `prefs_set.rs` (proposal §3.1 bug-class avoidance).
//!
//! FK posture: when the request carries `Some(tenant_id)`, the
//! verb validates the id resolves in [`TenantStore`] before
//! writing. Silently assigning a user to a nonexistent tenant
//! would let the user-admin surface drift out of sync with the
//! tenant directory.
//!
//! Audit posture: see the DTO module doc for the link to the
//! audit-floor mechanism that pins `user` `Change` rows past undo
//! retention.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::user::tenant_assign::{UserTenantAssignRequest, UserTenantAssignResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::tenant::store::TenantStore;
use crate::undo::dispatch::ReversibleTool;
use crate::user::store::{UserAdminStore, UserRow, USER_KIND};

/// Concrete [`Tool`] for `rubix.user.tenant.assign`.
pub struct UserTenantAssignTool {
    users: Arc<dyn UserAdminStore>,
    tenants: Arc<dyn TenantStore>,
}

impl UserTenantAssignTool {
    /// Wrap the shared stores. The tenant store is used to
    /// validate that the assignment target exists before the verb
    /// writes the user row; the user store carries the write.
    pub fn new(users: Arc<dyn UserAdminStore>, tenants: Arc<dyn TenantStore>) -> Self {
        Self { users, tenants }
    }
}

#[async_trait]
impl Tool for UserTenantAssignTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.user.tenant.assign".to_owned(),
            description: rubix_spi::dto::user::tenant_assign::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "user_id":   { "type": ["string", "null"] },
                    "email":     { "type": ["string", "null"] },
                    "tenant_id": { "type": ["string", "null"] }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: UserTenantAssignRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("UserTenantAssignRequest: {e}"),
            })?;

        // Reject the empty string explicitly. `Some("")` is almost
        // always a wire-shaped bug (a blank form field), and
        // accepting it silently would let it round-trip through
        // the store and corrupt the row. `None` (unassign) is the
        // only legal "no tenant" form.
        if let Some(id) = req.tenant_id.as_deref() {
            if id.is_empty() || id.trim() != id {
                return Err(Error::Invalid {
                    message: "UserTenantAssignRequest.tenant_id must be non-empty and trimmed"
                        .to_owned(),
                });
            }
            // FK validation: refuse to assign to a tenant that
            // does not resolve. Cheap one-row read; the only
            // implementor today walks the full list, but
            // production PG impls will index on id.
            if self.tenants.get(id).await?.is_none() {
                return Err(Error::NotFound {
                    what: format!("tenant:{id}"),
                });
            }
        }

        let target = resolve_target(&*self.users, &req).await?;
        let (prior, new) = self
            .users
            .set_tenant(&target.user_id, req.tenant_id.clone())
            .await?;
        let was_unchanged = prior.tenant_id == new.tenant_id;

        let key = if was_unchanged {
            "rubix.user.tenant.unchanged"
        } else if new.tenant_id.is_some() {
            "rubix.user.tenant.assigned"
        } else {
            "rubix.user.tenant.unassigned"
        };
        let now_ms = now_epoch_ms();
        let mut diag = Diagnostic::new(MessageKey::parse(key).expect("hard-coded key parses"))
            .with_param("email", DiagnosticParam::String(new.email.clone()))
            .with_param("at", DiagnosticParam::Timestamp(now_ms));
        if let Some(t) = new.tenant_id.as_ref() {
            diag = diag.with_param("tenant", DiagnosticParam::String(t.clone()));
        }
        if !was_unchanged {
            if let Some(p) = prior.tenant_id.as_ref() {
                diag = diag.with_param("prior", DiagnosticParam::String(p.clone()));
            }
        }

        let response = UserTenantAssignResponse {
            summary: diag,
            user_id: new.user_id.clone(),
            email: new.email.clone(),
            prior_tenant_id: prior.tenant_id.clone(),
            new_tenant_id: new.tenant_id.clone(),
            was_unchanged,
            // Echo the row's other identity-bearing fields so
            // `change_for` reconstructs the full snapshot byte-
            // exact. tenant_assign doesn't touch any of these, so
            // both sides share them — pick `new` for clarity that
            // this is the post-mutation live value (identical to
            // `prior` for these fields).
            role: new.role,
            disabled_at_ms: new.disabled_at_ms,
            prefs_json: new.prefs_json,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for UserTenantAssignTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: UserTenantAssignResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_unchanged {
            // No state change — recording would let undo silently
            // flip an assignment the caller did not actually
            // change. Mirrors the `was_unchanged` short-circuit
            // in `UserRoleSetTool::change_for`.
            return None;
        }
        // Full `UserRow` snapshot reconstructed from the response.
        // tenant_assign doesn't touch `role`, `disabled_at_ms` or
        // `prefs_json`, so both sides share them. The only field
        // that flips is `tenant_id`.
        let before = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.role.clone(),
            disabled_at_ms: resp.disabled_at_ms,
            prefs_json: resp.prefs_json.clone(),
            tenant_id: resp.prior_tenant_id.clone(),
        };
        let after = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email.clone(),
            role: resp.role.clone(),
            disabled_at_ms: resp.disabled_at_ms,
            prefs_json: resp.prefs_json.clone(),
            tenant_id: resp.new_tenant_id.clone(),
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

async fn resolve_target(
    store: &dyn UserAdminStore,
    req: &UserTenantAssignRequest,
) -> Result<UserRow> {
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
        message: "UserTenantAssignRequest requires user_id or email".to_owned(),
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
    use crate::tenant::store::{InMemoryTenantStore, TenantRow};
    use crate::user::store::InMemoryUserStore;
    use serde_json::json;
    use starter_spi::changelog::Op;

    fn tenants() -> Arc<InMemoryTenantStore> {
        Arc::new(InMemoryTenantStore::seeded(vec![
            TenantRow {
                tenant_id: "t-acme".into(),
                name: "Acme".into(),
                locale: "en".into(),
            },
            TenantRow {
                tenant_id: "t-globex".into(),
                name: "Globex".into(),
                locale: "en".into(),
            },
        ]))
    }

    async fn seeded() -> (Arc<InMemoryUserStore>, Arc<InMemoryTenantStore>) {
        let users = Arc::new(InMemoryUserStore::new());
        users
            .create(UserRow {
                user_id: "u-1".into(),
                email: "ada@x".into(),
                role: "reader".into(),
                disabled_at_ms: None,
                prefs_json: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        (users, tenants())
    }

    #[tokio::test]
    async fn assign_on_blank_row_changes_none_to_some() {
        let (users, tenants) = seeded().await;
        let tool = UserTenantAssignTool::new(users, tenants);
        let out = tool
            .invoke(json!({"email": "ada@x", "tenant_id": "t-acme"}))
            .await
            .unwrap();
        let resp: UserTenantAssignResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.tenant.assigned");
        assert!(resp.prior_tenant_id.is_none());
        assert_eq!(resp.new_tenant_id, Some("t-acme".to_owned()));
        assert!(!resp.was_unchanged);
    }

    #[tokio::test]
    async fn unassign_clears_to_none_and_emits_unassigned() {
        let (users, tenants) = seeded().await;
        let tool = UserTenantAssignTool::new(users, tenants);
        // First assign...
        let _ = tool
            .invoke(json!({"user_id": "u-1", "tenant_id": "t-acme"}))
            .await
            .unwrap();
        // ...then unassign.
        let out = tool
            .invoke(json!({"user_id": "u-1", "tenant_id": null}))
            .await
            .unwrap();
        let resp: UserTenantAssignResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.tenant.unassigned");
        assert_eq!(resp.prior_tenant_id, Some("t-acme".to_owned()));
        assert!(resp.new_tenant_id.is_none());
        assert!(!resp.was_unchanged);
    }

    #[tokio::test]
    async fn assign_same_tenant_is_noop_and_skips_draft() {
        let (users, tenants) = seeded().await;
        let tool = UserTenantAssignTool::new(users, tenants);
        let input = json!({"user_id": "u-1", "tenant_id": "t-acme"});
        let _ = tool.invoke(input.clone()).await.unwrap();
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: UserTenantAssignResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.tenant.unchanged");
        assert!(resp.was_unchanged);
        assert!(
            tool.change_for(&input, &out).is_none(),
            "no-op tenant-assign must not record a Change",
        );
    }

    #[tokio::test]
    async fn unassign_when_already_unassigned_is_noop_and_skips_draft() {
        let (users, tenants) = seeded().await;
        let tool = UserTenantAssignTool::new(users.clone(), tenants);
        let input = json!({"user_id": "u-1", "tenant_id": null});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: UserTenantAssignResponse = serde_json::from_value(out.clone()).unwrap();
        // Row is born unassigned — None → None is a no-op.
        assert_eq!(resp.summary.code.as_str(), "rubix.user.tenant.unchanged");
        assert!(resp.was_unchanged);
        assert!(tool.change_for(&input, &out).is_none());
    }

    #[tokio::test]
    async fn assigning_to_unknown_tenant_returns_not_found() {
        let (users, tenants) = seeded().await;
        let tool = UserTenantAssignTool::new(users.clone(), tenants);
        let err = tool
            .invoke(json!({"user_id": "u-1", "tenant_id": "t-ghost"}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::NotFound { ref what } if what == "tenant:t-ghost"),
            "unknown tenant must NotFound; got {err:?}",
        );
        // And the user row must NOT have flipped.
        let row = users.get("u-1").await.unwrap().unwrap();
        assert!(
            row.tenant_id.is_none(),
            "failed FK must not mutate user row"
        );
    }

    #[tokio::test]
    async fn empty_tenant_id_is_rejected() {
        let (users, tenants) = seeded().await;
        let tool = UserTenantAssignTool::new(users, tenants);
        let err = tool
            .invoke(json!({"user_id": "u-1", "tenant_id": ""}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn missing_target_returns_not_found() {
        let (users, tenants) = seeded().await;
        let tool = UserTenantAssignTool::new(users, tenants);
        let err = tool
            .invoke(json!({"email": "missing@x", "tenant_id": "t-acme"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn change_for_records_update_with_before_after_snapshots() {
        let (users, tenants) = seeded().await;
        let tool = UserTenantAssignTool::new(users, tenants);
        let input = json!({"user_id": "u-1", "tenant_id": "t-acme"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft present");
        assert!(matches!(draft.op, Op::Update));
        let before: UserRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: UserRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert!(before.tenant_id.is_none());
        assert_eq!(after.tenant_id, Some("t-acme".to_owned()));
        // The other identity-bearing fields round-trip — pins the
        // contract that the snapshot is the full row, not a delta.
        assert_eq!(before.role, after.role);
        assert_eq!(before.email, after.email);
        assert_eq!(before.disabled_at_ms, after.disabled_at_ms);
        assert_eq!(before.prefs_json, after.prefs_json);
    }

    #[tokio::test]
    async fn snapshot_byte_exact_preserves_role_disabled_and_prefs() {
        // Load-bearing for the §3.1-bug-class avoidance: a tenant
        // change on an admin-role, disabled, prefs-bearing user
        // must not surface a snapshot that silently re-enables
        // them, downgrades their role, or clears their prefs on
        // undo. Mirrors the equivalent test on `prefs_set.rs` for
        // the symmetric verb.
        let users = Arc::new(InMemoryUserStore::new());
        users
            .create(UserRow {
                user_id: "u-1".into(),
                email: "bob@x".into(),
                role: "admin".into(),
                disabled_at_ms: Some(1_700_000_000_000),
                prefs_json: Some(json!({"locale": "es-ES"})),
                tenant_id: None,
            })
            .await
            .unwrap();

        let tool = UserTenantAssignTool::new(users, tenants());
        let input = json!({"user_id": "u-1", "tenant_id": "t-acme"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft");
        let before: UserRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: UserRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(before.role, "admin");
        assert_eq!(after.role, "admin");
        assert_eq!(before.disabled_at_ms, Some(1_700_000_000_000));
        assert_eq!(after.disabled_at_ms, Some(1_700_000_000_000));
        assert_eq!(before.prefs_json, Some(json!({"locale": "es-ES"})));
        assert_eq!(after.prefs_json, Some(json!({"locale": "es-ES"})));
        // The actual flip:
        assert!(before.tenant_id.is_none());
        assert_eq!(after.tenant_id, Some("t-acme".to_owned()));
    }
}
