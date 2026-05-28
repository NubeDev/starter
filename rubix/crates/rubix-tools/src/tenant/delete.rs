//! `rubix.tenant.delete` — tool dispatch.
//!
//! Deletes an existing tenant via the shared [`TenantStore`].
//! Refuses with `rubix.tenant.has_users` when any user is
//! currently assigned to the tenant — the operator must
//! `rubix.user.tenant.assign` those users elsewhere first.
//!
//! See the DTO module doc for the cascade decision rationale.
//!
//! Snapshot shape: `Op::Delete`, `before` = the full prior
//! [`TenantRow`] (so undo can re-create it), `after = None`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::tenant::delete::{TenantDeleteRequest, TenantDeleteResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::tenant::store::{TenantRow, TenantStore, TENANT_KIND};
use crate::undo::dispatch::ReversibleTool;
use crate::user::store::UserAdminStore;

/// Concrete [`Tool`] for `rubix.tenant.delete`.
pub struct TenantDeleteTool {
    tenants: Arc<dyn TenantStore>,
    users: Arc<dyn UserAdminStore>,
}

impl TenantDeleteTool {
    /// Wrap the shared stores. The user store is used to enforce
    /// the refuse-if-users-assigned cascade decision before the
    /// tenant store mutates.
    pub fn new(tenants: Arc<dyn TenantStore>, users: Arc<dyn UserAdminStore>) -> Self {
        Self { tenants, users }
    }
}

#[async_trait]
impl Tool for TenantDeleteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.tenant.delete".to_owned(),
            description: rubix_spi::dto::tenant::delete::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id": { "type": "string", "minLength": 1 }
                },
                "required": ["tenant_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: TenantDeleteRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("TenantDeleteRequest: {e}"),
            })?;

        if req.tenant_id.is_empty() || req.tenant_id.trim() != req.tenant_id {
            return Err(Error::Invalid {
                message: "TenantDeleteRequest.tenant_id must be non-empty and trimmed".to_owned(),
            });
        }

        // Resolve the row before doing any work so we can return a
        // structured NotFound (rather than the store's lower-level
        // message) and so the snapshot we record on success has
        // the row's name/locale.
        let prior = self
            .tenants
            .get(&req.tenant_id)
            .await?
            .ok_or_else(|| Error::NotFound {
                what: format!("tenant:{}", req.tenant_id),
            })?;

        // Cascade check: refuse if any user is still assigned.
        // See the DTO module doc for the rationale and the
        // alternatives considered. We surface the count so the
        // operator can run `rubix.user.list` and clean up
        // explicitly.
        let assigned_count = self
            .users
            .list()
            .await?
            .into_iter()
            .filter(|u| u.tenant_id.as_deref() == Some(req.tenant_id.as_str()))
            .count();
        if assigned_count > 0 {
            let diag = Diagnostic::new(
                MessageKey::parse("rubix.tenant.has_users").expect("hard-coded key parses"),
            )
            .with_param("tenant", DiagnosticParam::String(req.tenant_id.clone()))
            .with_param("name", DiagnosticParam::String(prior.name.clone()))
            .with_param("count", DiagnosticParam::I64(assigned_count as i64));
            return Err(Error::Conflict {
                message: serde_json::to_string(&diag).unwrap_or_else(|_| {
                    format!(
                        "tenant {} has {assigned_count} users assigned; unassign first",
                        req.tenant_id,
                    )
                }),
            });
        }

        self.tenants.delete(&req.tenant_id).await?;
        let deleted_at_ms = now_epoch_ms();
        let summary = Diagnostic::new(
            MessageKey::parse("rubix.tenant.deleted").expect("hard-coded key parses"),
        )
        .with_param("name", DiagnosticParam::String(prior.name.clone()))
        .with_param("tenant", DiagnosticParam::String(prior.tenant_id.clone()))
        .with_param("at", DiagnosticParam::Timestamp(deleted_at_ms));

        let response = TenantDeleteResponse {
            summary,
            tenant_id: prior.tenant_id,
            name: prior.name,
            locale: prior.locale,
            deleted_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for TenantDeleteTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: TenantDeleteResponse = serde_json::from_value(output.clone()).ok()?;
        // Full-row `before` snapshot reconstructed from the
        // response — `change_for` does not re-read the store
        // because the row no longer exists after the delete
        // succeeded. Every identity-bearing field rides on the
        // response for this reason (same posture as the user
        // verbs after the §3.1 bug-class fix).
        let before = TenantRow {
            tenant_id: resp.tenant_id.clone(),
            name: resp.name,
            locale: resp.locale,
        };
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: TENANT_KIND.into(),
                id: Some(resp.tenant_id),
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
    use crate::tenant::store::InMemoryTenantStore;
    use crate::user::store::{InMemoryUserStore, UserRow};
    use serde_json::json;

    fn tenant(id: &str, name: &str) -> TenantRow {
        TenantRow {
            tenant_id: id.into(),
            name: name.into(),
            locale: "en".into(),
        }
    }

    fn user_in_tenant(id: &str, email: &str, tenant_id: Option<&str>) -> UserRow {
        UserRow {
            user_id: id.into(),
            email: email.into(),
            role: "reader".into(),
            disabled_at_ms: None,
            prefs_json: None,
            tenant_id: tenant_id.map(str::to_owned),
        }
    }

    async fn seeded() -> (Arc<InMemoryTenantStore>, Arc<InMemoryUserStore>) {
        let tenants = Arc::new(InMemoryTenantStore::new());
        tenants.create(tenant("t-acme", "Acme")).await.unwrap();
        tenants.create(tenant("t-globex", "Globex")).await.unwrap();
        let users = Arc::new(InMemoryUserStore::new());
        (tenants, users)
    }

    #[tokio::test]
    async fn delete_unassigned_tenant_succeeds() {
        let (tenants, users) = seeded().await;
        let tool = TenantDeleteTool::new(tenants.clone(), users);
        let out = tool
            .invoke(json!({"tenant_id": "t-acme"}))
            .await
            .unwrap();
        let resp: TenantDeleteResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.tenant.deleted");
        assert_eq!(resp.tenant_id, "t-acme");
        assert!(tenants.get("t-acme").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_refuses_when_users_assigned() {
        let (tenants, users) = seeded().await;
        users
            .create(user_in_tenant("u-1", "ada@x", Some("t-acme")))
            .await
            .unwrap();
        let tool = TenantDeleteTool::new(tenants.clone(), users);
        let err = tool
            .invoke(json!({"tenant_id": "t-acme"}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::Conflict { .. }),
            "delete must refuse with Conflict; got {err:?}",
        );
        // And the row must still be there.
        assert!(tenants.get("t-acme").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn delete_succeeds_after_users_unassigned() {
        let (tenants, users) = seeded().await;
        users
            .create(user_in_tenant("u-1", "ada@x", Some("t-acme")))
            .await
            .unwrap();
        // Unassign.
        users
            .set_tenant("u-1", None)
            .await
            .unwrap();
        let tool = TenantDeleteTool::new(tenants.clone(), users);
        let out = tool
            .invoke(json!({"tenant_id": "t-acme"}))
            .await
            .unwrap();
        let resp: TenantDeleteResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.tenant.deleted");
        assert!(tenants.get("t-acme").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_only_blocks_for_target_tenant_assignments() {
        // A user assigned to t-globex must NOT block deletion of
        // t-acme — the filter is per-tenant, not "any user has a
        // tenant_id".
        let (tenants, users) = seeded().await;
        users
            .create(user_in_tenant("u-1", "ada@x", Some("t-globex")))
            .await
            .unwrap();
        let tool = TenantDeleteTool::new(tenants.clone(), users);
        let _ = tool
            .invoke(json!({"tenant_id": "t-acme"}))
            .await
            .unwrap();
        assert!(tenants.get("t-acme").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_tenant_returns_not_found() {
        let (tenants, users) = seeded().await;
        let tool = TenantDeleteTool::new(tenants, users);
        let err = tool
            .invoke(json!({"tenant_id": "t-ghost"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn empty_tenant_id_is_rejected() {
        let (tenants, users) = seeded().await;
        let tool = TenantDeleteTool::new(tenants, users);
        let err = tool
            .invoke(json!({"tenant_id": ""}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn change_for_records_delete_with_full_before_snapshot() {
        let (tenants, users) = seeded().await;
        let tool = TenantDeleteTool::new(tenants, users);
        let input = json!({"tenant_id": "t-acme"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft present");
        assert!(matches!(draft.op, Op::Delete));
        let before: TenantRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        assert_eq!(before.tenant_id, "t-acme");
        assert_eq!(before.name, "Acme");
        assert_eq!(before.locale, "en");
        assert!(
            draft.after.is_none(),
            "delete snapshot's `after` must be None",
        );
    }
}
