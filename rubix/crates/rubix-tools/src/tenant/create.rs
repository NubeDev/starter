//! `rubix.tenant.create` — tool dispatch.
//!
//! Provisions a new tenant via the shared [`TenantStore`]. The
//! successful response carries a `Diagnostic` keyed
//! `rubix.tenant.created`. The companion `change_for` impl
//! produces a snapshot-shaped [`ChangeDraft`] (Op::Create,
//! `after` = [`TenantRow`] JSON) so the undo dispatcher walks it
//! back through [`super::store::TenantReversible`].

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::tenant::create::{TenantCreateRequest, TenantCreateResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;
use uuid::Uuid;

use crate::tenant::store::{TenantRow, TenantStore, TENANT_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.tenant.create`.
pub struct TenantCreateTool {
    store: Arc<dyn TenantStore>,
}

impl TenantCreateTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TenantStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TenantCreateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.tenant.create".to_owned(),
            description: rubix_spi::dto::tenant::create::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id": { "type": ["string", "null"] },
                    "name":      { "type": "string", "minLength": 1 },
                    "locale":    { "type": ["string", "null"] }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: TenantCreateRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("TenantCreateRequest: {e}"),
            })?;

        // Trim-check the name. Same posture as role_set: leading/
        // trailing whitespace would compare unequal to a stored
        // form on a future rename, and would silently bifurcate
        // the operator-facing tenant taxonomy.
        if req.name.is_empty() || req.name.trim() != req.name {
            return Err(Error::Invalid {
                message: "TenantCreateRequest.name must be non-empty and trimmed".to_owned(),
            });
        }
        // Explicit empty `tenant_id` / `locale` are rejected for
        // the same reason as the empty-string rule in
        // `rubix.user.tenant.assign` — `Some("")` is almost
        // always a wire-shaped bug.
        if let Some(id) = req.tenant_id.as_deref() {
            if id.is_empty() || id.trim() != id {
                return Err(Error::Invalid {
                    message: "TenantCreateRequest.tenant_id must be non-empty and trimmed"
                        .to_owned(),
                });
            }
        }
        if let Some(loc) = req.locale.as_deref() {
            if loc.is_empty() || loc.trim() != loc {
                return Err(Error::Invalid {
                    message: "TenantCreateRequest.locale must be non-empty and trimmed".to_owned(),
                });
            }
        }

        let tenant_id = req
            .tenant_id
            .unwrap_or_else(|| format!("t-{}", Uuid::new_v4().simple()));
        let locale = req.locale.unwrap_or_else(|| "en".to_owned());
        let created_at_ms = now_epoch_ms();

        let row = TenantRow {
            tenant_id: tenant_id.clone(),
            name: req.name.clone(),
            locale: locale.clone(),
        };
        let row = self.store.create(row).await?;

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.tenant.created").expect("hard-coded key parses"),
        )
        .with_param("name", DiagnosticParam::String(row.name.clone()))
        .with_param("tenant", DiagnosticParam::String(row.tenant_id.clone()))
        .with_param("at", DiagnosticParam::Timestamp(created_at_ms));

        let response = TenantCreateResponse {
            summary,
            tenant_id: row.tenant_id,
            name: row.name,
            locale: row.locale,
            created_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for TenantCreateTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: TenantCreateResponse = serde_json::from_value(output.clone()).ok()?;
        let row = TenantRow {
            tenant_id: resp.tenant_id.clone(),
            name: resp.name,
            locale: resp.locale,
        };
        let after = serde_json::to_value(&row).ok()?;
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: TENANT_KIND.into(),
                id: Some(row.tenant_id),
                owner: None,
                tenant: None,
            },
            op: Op::Create,
            before: None,
            after: Some(after),
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
    use serde_json::json;

    #[tokio::test]
    async fn create_emits_created_diagnostic_and_persists_row() {
        let store = Arc::new(InMemoryTenantStore::new());
        let tool = TenantCreateTool::new(store.clone());
        let out = tool
            .invoke(json!({"name": "Acme", "locale": "en"}))
            .await
            .unwrap();
        let resp: TenantCreateResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.tenant.created");
        assert!(resp.tenant_id.starts_with("t-"));
        let row = store.get(&resp.tenant_id).await.unwrap().unwrap();
        assert_eq!(row.name, "Acme");
        assert_eq!(row.locale, "en");
    }

    #[tokio::test]
    async fn explicit_id_is_honoured() {
        let store = Arc::new(InMemoryTenantStore::new());
        let tool = TenantCreateTool::new(store.clone());
        let out = tool
            .invoke(json!({"tenant_id": "t-acme", "name": "Acme"}))
            .await
            .unwrap();
        let resp: TenantCreateResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.tenant_id, "t-acme");
        assert_eq!(resp.locale, "en", "locale defaults to en when omitted");
    }

    #[tokio::test]
    async fn duplicate_id_is_rejected() {
        let store = Arc::new(InMemoryTenantStore::new());
        let tool = TenantCreateTool::new(store);
        let _ = tool
            .invoke(json!({"tenant_id": "t-1", "name": "Acme"}))
            .await
            .unwrap();
        let err = tool
            .invoke(json!({"tenant_id": "t-1", "name": "Globex"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Conflict { .. }));
    }

    #[tokio::test]
    async fn duplicate_name_is_rejected() {
        let store = Arc::new(InMemoryTenantStore::new());
        let tool = TenantCreateTool::new(store);
        let _ = tool.invoke(json!({"name": "Acme"})).await.unwrap();
        let err = tool.invoke(json!({"name": "Acme"})).await.unwrap_err();
        assert!(matches!(err, Error::Conflict { .. }));
    }

    #[tokio::test]
    async fn empty_name_is_rejected() {
        let store = Arc::new(InMemoryTenantStore::new());
        let tool = TenantCreateTool::new(store);
        let err = tool.invoke(json!({"name": ""})).await.unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn untrimmed_name_is_rejected() {
        let store = Arc::new(InMemoryTenantStore::new());
        let tool = TenantCreateTool::new(store);
        let err = tool.invoke(json!({"name": " Acme "})).await.unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn change_for_returns_create_draft_with_tenant_row_after() {
        let store = Arc::new(InMemoryTenantStore::new());
        let tool = TenantCreateTool::new(store);
        let input = json!({"name": "Acme"});
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert_eq!(draft.resource.kind, TENANT_KIND);
        assert!(matches!(draft.op, Op::Create));
        let row: TenantRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(row.name, "Acme");
        assert_eq!(row.locale, "en");
    }
}
