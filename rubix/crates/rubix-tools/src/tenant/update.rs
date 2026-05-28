//! `rubix.tenant.update` — tool dispatch.
//!
//! Renames a tenant and/or changes its locale. Idempotent on a
//! per-field basis (a field that already matches its requested
//! value contributes nothing; if all requested fields match, the
//! whole call collapses to the `rubix.tenant.unchanged`
//! diagnostic and produces no `ChangeDraft`). See the DTO
//! module doc for the rest of the contract.
//!
//! Uniqueness on rename: the verb walks `store.list()` to confirm
//! no *other* tenant row already carries the requested name. We
//! deliberately keep this check at the verb level (rather than
//! adding a third store method) so PG-backed implementations can
//! enforce the same invariant via a partial unique index without
//! the store trait carrying update-flavoured behaviour. The
//! in-memory store's `put` bypasses uniqueness (snapshot restore
//! must succeed); the verb is the gatekeeper.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::tenant::update::{TenantUpdateRequest, TenantUpdateResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::tenant::store::{TenantRow, TenantStore, TENANT_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.tenant.update`.
pub struct TenantUpdateTool {
    store: Arc<dyn TenantStore>,
}

impl TenantUpdateTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TenantStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TenantUpdateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.tenant.update".to_owned(),
            description: rubix_spi::dto::tenant::update::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id": { "type": "string", "minLength": 1 },
                    "name":      { "type": ["string", "null"] },
                    "locale":    { "type": ["string", "null"] }
                },
                "required": ["tenant_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: TenantUpdateRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("TenantUpdateRequest: {e}"),
            })?;

        if req.tenant_id.is_empty() || req.tenant_id.trim() != req.tenant_id {
            return Err(Error::Invalid {
                message: "TenantUpdateRequest.tenant_id must be non-empty and trimmed".to_owned(),
            });
        }
        // Refuse a no-op request shape. A request with both
        // fields None is almost always a wire-shaped bug; the
        // "really meant unchanged" case is handled below via
        // value-equality once the row is loaded.
        if req.name.is_none() && req.locale.is_none() {
            return Err(Error::Invalid {
                message: "TenantUpdateRequest requires at least one of name / locale".to_owned(),
            });
        }
        if let Some(name) = req.name.as_deref() {
            if name.is_empty() || name.trim() != name {
                return Err(Error::Invalid {
                    message: "TenantUpdateRequest.name must be non-empty and trimmed".to_owned(),
                });
            }
        }
        if let Some(loc) = req.locale.as_deref() {
            if loc.is_empty() || loc.trim() != loc {
                return Err(Error::Invalid {
                    message: "TenantUpdateRequest.locale must be non-empty and trimmed".to_owned(),
                });
            }
        }

        let prior =
            self.store
                .get(&req.tenant_id)
                .await?
                .ok_or_else(|| Error::NotFound {
                    what: format!("tenant:{}", req.tenant_id),
                })?;

        let new_name = req.name.clone().unwrap_or_else(|| prior.name.clone());
        let new_locale = req.locale.clone().unwrap_or_else(|| prior.locale.clone());

        let was_unchanged = new_name == prior.name && new_locale == prior.locale;
        let updated_at_ms = now_epoch_ms();

        if !was_unchanged {
            // Uniqueness check on rename: only when the name
            // actually flipped, and only against *other* rows.
            if new_name != prior.name {
                let collision = self
                    .store
                    .list()
                    .await?
                    .into_iter()
                    .any(|r| r.tenant_id != prior.tenant_id && r.name == new_name);
                if collision {
                    return Err(Error::Conflict {
                        message: format!("tenant with name {new_name} already exists"),
                    });
                }
            }
            let next = TenantRow {
                tenant_id: prior.tenant_id.clone(),
                name: new_name.clone(),
                locale: new_locale.clone(),
            };
            self.store.put(next).await?;
        }

        let key = if was_unchanged {
            "rubix.tenant.unchanged"
        } else {
            "rubix.tenant.updated"
        };
        let mut diag = Diagnostic::new(MessageKey::parse(key).expect("hard-coded key parses"))
            .with_param("tenant", DiagnosticParam::String(prior.tenant_id.clone()))
            .with_param("name", DiagnosticParam::String(new_name.clone()))
            .with_param("at", DiagnosticParam::Timestamp(updated_at_ms));
        if !was_unchanged {
            diag = diag
                .with_param("prior_name", DiagnosticParam::String(prior.name.clone()))
                .with_param("prior_locale", DiagnosticParam::String(prior.locale.clone()))
                .with_param("new_locale", DiagnosticParam::String(new_locale.clone()));
        }

        let response = TenantUpdateResponse {
            summary: diag,
            tenant_id: prior.tenant_id,
            prior_name: prior.name,
            new_name,
            prior_locale: prior.locale,
            new_locale,
            was_unchanged,
            updated_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for TenantUpdateTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: TenantUpdateResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_unchanged {
            // No state change — recording would let undo silently
            // rewrite a row that the caller did not actually flip.
            return None;
        }
        let before = TenantRow {
            tenant_id: resp.tenant_id.clone(),
            name: resp.prior_name,
            locale: resp.prior_locale,
        };
        let after = TenantRow {
            tenant_id: resp.tenant_id.clone(),
            name: resp.new_name,
            locale: resp.new_locale,
        };
        Some(ChangeDraft::update(
            ResourceRef {
                kind: TENANT_KIND.into(),
                id: Some(resp.tenant_id),
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
    use crate::tenant::store::InMemoryTenantStore;
    use serde_json::json;
    use starter_spi::changelog::Op;

    fn row(id: &str, name: &str, locale: &str) -> TenantRow {
        TenantRow {
            tenant_id: id.into(),
            name: name.into(),
            locale: locale.into(),
        }
    }

    async fn seeded() -> Arc<InMemoryTenantStore> {
        let store = Arc::new(InMemoryTenantStore::new());
        store.create(row("t-1", "Acme", "en")).await.unwrap();
        store.create(row("t-2", "Globex", "en")).await.unwrap();
        store
    }

    #[tokio::test]
    async fn rename_changes_name_and_emits_updated() {
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store.clone());
        let out = tool
            .invoke(json!({"tenant_id": "t-1", "name": "Acme Corp"}))
            .await
            .unwrap();
        let resp: TenantUpdateResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.tenant.updated");
        assert!(!resp.was_unchanged);
        assert_eq!(resp.prior_name, "Acme");
        assert_eq!(resp.new_name, "Acme Corp");
        assert_eq!(resp.prior_locale, "en");
        assert_eq!(resp.new_locale, "en");
        let row = store.get("t-1").await.unwrap().unwrap();
        assert_eq!(row.name, "Acme Corp");
    }

    #[tokio::test]
    async fn relocale_changes_locale_and_leaves_name_alone() {
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store.clone());
        let out = tool
            .invoke(json!({"tenant_id": "t-1", "locale": "es"}))
            .await
            .unwrap();
        let resp: TenantUpdateResponse = serde_json::from_value(out).unwrap();
        assert!(!resp.was_unchanged);
        assert_eq!(resp.new_name, "Acme");
        assert_eq!(resp.new_locale, "es");
        let row = store.get("t-1").await.unwrap().unwrap();
        assert_eq!(row.name, "Acme");
        assert_eq!(row.locale, "es");
    }

    #[tokio::test]
    async fn rename_and_relocale_together_apply_both() {
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store.clone());
        let out = tool
            .invoke(json!({"tenant_id": "t-1", "name": "Acme S.A.", "locale": "es"}))
            .await
            .unwrap();
        let resp: TenantUpdateResponse = serde_json::from_value(out).unwrap();
        assert!(!resp.was_unchanged);
        assert_eq!(resp.new_name, "Acme S.A.");
        assert_eq!(resp.new_locale, "es");
    }

    #[tokio::test]
    async fn rename_to_existing_name_is_rejected_as_conflict() {
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store);
        let err = tool
            .invoke(json!({"tenant_id": "t-1", "name": "Globex"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Conflict { .. }));
    }

    #[tokio::test]
    async fn rename_to_same_name_is_unchanged_and_skips_draft() {
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store);
        let input = json!({"tenant_id": "t-1", "name": "Acme"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: TenantUpdateResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.tenant.unchanged");
        assert!(resp.was_unchanged);
        assert_eq!(resp.prior_name, "Acme");
        assert_eq!(resp.new_name, "Acme");
        let draft = tool.change_for(&input, &out);
        assert!(
            draft.is_none(),
            "unchanged path must not produce a ChangeDraft"
        );
    }

    #[tokio::test]
    async fn renaming_to_own_current_name_does_not_self_collide() {
        // Regression guard: the uniqueness check must exclude
        // the row being updated. Without the `r.tenant_id !=
        // prior.tenant_id` filter, "rename Acme to Acme" would
        // false-positive as a conflict on the unchanged path,
        // and "rename Acme to Acme" via Some("Acme") would loop
        // through the conflict branch.
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store);
        // First a relocale that forces the verb past the
        // unchanged short-circuit but keeps the name the same.
        let out = tool
            .invoke(json!({"tenant_id": "t-1", "name": "Acme", "locale": "es"}))
            .await
            .unwrap();
        let resp: TenantUpdateResponse = serde_json::from_value(out).unwrap();
        assert!(!resp.was_unchanged);
        assert_eq!(resp.new_name, "Acme");
        assert_eq!(resp.new_locale, "es");
    }

    #[tokio::test]
    async fn missing_tenant_returns_not_found() {
        let store = Arc::new(InMemoryTenantStore::new());
        let tool = TenantUpdateTool::new(store);
        let err = tool
            .invoke(json!({"tenant_id": "t-ghost", "name": "Anything"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn empty_tenant_id_is_rejected() {
        let store = Arc::new(InMemoryTenantStore::new());
        let tool = TenantUpdateTool::new(store);
        let err = tool
            .invoke(json!({"tenant_id": "", "name": "Acme"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn no_fields_supplied_is_rejected() {
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store);
        let err = tool
            .invoke(json!({"tenant_id": "t-1"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn empty_name_is_rejected() {
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store);
        let err = tool
            .invoke(json!({"tenant_id": "t-1", "name": ""}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn untrimmed_locale_is_rejected() {
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store);
        let err = tool
            .invoke(json!({"tenant_id": "t-1", "locale": " es "}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn change_for_records_update_with_before_after_snapshots() {
        let store = seeded().await;
        let tool = TenantUpdateTool::new(store);
        let input = json!({"tenant_id": "t-1", "name": "Acme Corp", "locale": "es"});
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert_eq!(draft.resource.kind, TENANT_KIND);
        assert_eq!(draft.resource.id.as_deref(), Some("t-1"));
        assert!(matches!(draft.op, Op::Update));
        let before: TenantRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: TenantRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(before, row("t-1", "Acme", "en"));
        assert_eq!(after, row("t-1", "Acme Corp", "es"));
    }
}
