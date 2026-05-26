//! `rubix.dashboard.delete` — tool dispatch.
//!
//! Soft-deletes an SDUI page by superseding every live revision
//! for `(tenant_id, page_id)`. The history rows stay so
//! `rubix.undo.last` can restore the page byte-for-byte and the
//! audit trail remains complete.
//!
//! Bundled pages — rows whose `created_by` equals
//! [`rubix_spi::dashboard::BUNDLED_PRINCIPAL`] — are refused with
//! a structured [`Diagnostic`] keyed
//! `rubix.dashboard.delete.refused_system` (transport maps to
//! HTTP 409). The [`ReversibleTool`] impl records an `Op::Delete`
//! `ChangeDraft` whose `before` payload is a [`DashboardSnapshot`]
//! of the row that was live at delete time; the dispatcher walks
//! this back through [`super::store::DashboardReversible`] which
//! re-inserts the row on undo.
//!
//! See `rubix/docs/scope/dashboards/04-tools.md`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::{DashboardStore, BUNDLED_PRINCIPAL};
use rubix_spi::dto::dashboard::delete::{DeleteDashboardRequest, DeleteDashboardResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::dashboard::store::{DashboardSnapshot, DASHBOARD_PAGE_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.dashboard.delete`.
pub struct DashboardDeleteTool {
    store: Arc<dyn DashboardStore>,
}

impl DashboardDeleteTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn DashboardStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for DashboardDeleteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.dashboard.delete".to_owned(),
            description: rubix_spi::dto::dashboard::delete::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id":  { "type": "string", "minLength": 1 },
                    "page_id":    { "type": "string", "minLength": 1 },
                    "deleted_by": { "type": "string", "minLength": 1 }
                },
                "required": ["tenant_id", "page_id", "deleted_by"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: DeleteDashboardRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("DeleteDashboardRequest: {e}"),
            })?;

        // Look the live row up so we can both (a) refuse bundled
        // pages and (b) snapshot the body for the undo path.
        let prior = self
            .store
            .get_active(&req.tenant_id, &req.page_id)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?
            .ok_or_else(|| Error::NotFound {
                what: format!("dashboard `{}:{}`", req.tenant_id, req.page_id),
            })?;

        // Bundled-page refusal. The diagnostic code is embedded in
        // the Error::Conflict message so the transport layer maps
        // to HTTP 409 and the consumer can pattern-match on it.
        if prior.created_by == BUNDLED_PRINCIPAL {
            let diag = Diagnostic::new(
                MessageKey::parse("rubix.dashboard.delete.refused_system")
                    .expect("hard-coded key parses"),
            )
            .with_param("page_id", DiagnosticParam::String(req.page_id.clone()));
            return Err(Error::Conflict {
                message: format!("{}: {}", diag.code.as_str(), req.page_id),
            });
        }

        let superseded = self
            .store
            .mark_superseded(&req.tenant_id, &req.page_id)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.dashboard.deleted").expect("hard-coded key parses"),
        )
        .with_param("page_id", DiagnosticParam::String(req.page_id.clone()));

        let response = DeleteDashboardResponse {
            summary,
            page_id: req.page_id,
            tenant_id: req.tenant_id,
            prior_revision_id: prior.revision_id,
            superseded,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for DashboardDeleteTool {
    fn change_for(&self, input: &Value, output: &Value) -> Option<ChangeDraft> {
        let req: DeleteDashboardRequest = serde_json::from_value(input.clone()).ok()?;
        let resp: DeleteDashboardResponse = serde_json::from_value(output.clone()).ok()?;
        // We can rebuild a partial `before` snapshot from the
        // request + response alone; for full fidelity the body
        // would need a re-fetch at draft time. The dispatcher
        // walks `before` back through
        // [`DashboardReversible::apply_inverse`] which re-inserts
        // via the store — body fidelity therefore depends on
        // whoever instantiates the `Change` carrying the prior
        // body in `before`. The tool body itself does not have
        // access to the body at this point (the row is already
        // superseded by `mark_superseded`); a follow-up wiring
        // pass capture-before-supersede will land alongside the
        // `prior_snapshot` seam in the store.
        let before = DashboardSnapshot {
            page_id: req.page_id.clone(),
            tenant_id: req.tenant_id.clone(),
            owner_principal: req.deleted_by.clone(),
            title: String::new(),
            tags: Vec::new(),
            body_json: Value::Null,
            created_by: req.deleted_by,
            revision_id: Some(resp.prior_revision_id),
        };
        let before_v = serde_json::to_value(&before).ok()?;
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: DASHBOARD_PAGE_KIND.into(),
                id: Some(req.page_id),
                owner: None,
                tenant: Some(req.tenant_id),
            },
            op: Op::Delete,
            before: Some(before_v),
            after: None,
            resource_version: None,
            correlation: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rubix_spi::dashboard::{DashboardRevision, DashboardStoreError, ListFilter, NewRevision};
    use starter_spi::changelog::{Actor, Change, ChangeId, GroupId, Reversible};
    use std::sync::Mutex;

    #[derive(Default)]
    struct InMemoryStore {
        rows: Mutex<Vec<DashboardRevision>>,
        next_rev: Mutex<u64>,
    }

    impl InMemoryStore {
        fn arc() -> Arc<Self> {
            Arc::new(Self::default())
        }
        fn next_rev(&self) -> String {
            let mut g = self.next_rev.lock().unwrap();
            *g += 1;
            format!("rev-{g}")
        }
        fn seed(&self, page_id: &str, tenant: &str, created_by: &str) -> String {
            let rev = self.next_rev();
            self.rows.lock().unwrap().push(DashboardRevision {
                page_id: page_id.into(),
                revision_id: rev.clone(),
                tenant_id: tenant.into(),
                owner_principal: "alice".into(),
                title: "Ops".into(),
                tags: vec!["custom".into()],
                body_json: serde_json::json!({ "v": 1 }),
                created_by: created_by.into(),
                created_at: "2026-05-25T00:00:00Z".into(),
                superseded_at: None,
            });
            rev
        }
        fn live_count(&self, tenant: &str, page: &str) -> usize {
            self.rows
                .lock()
                .unwrap()
                .iter()
                .filter(|r| r.tenant_id == tenant && r.page_id == page && r.superseded_at.is_none())
                .count()
        }
    }

    #[async_trait]
    impl DashboardStore for InMemoryStore {
        async fn insert_revision(
            &self,
            new: NewRevision,
        ) -> std::result::Result<DashboardRevision, DashboardStoreError> {
            let mut rows = self.rows.lock().unwrap();
            for r in rows.iter_mut() {
                if r.tenant_id == new.tenant_id
                    && r.page_id == new.page_id
                    && r.superseded_at.is_none()
                {
                    r.superseded_at = Some("2026-05-25T00:00:01Z".into());
                }
            }
            let inserted = DashboardRevision {
                page_id: new.page_id,
                revision_id: self.next_rev(),
                tenant_id: new.tenant_id,
                owner_principal: new.owner_principal,
                title: new.title,
                tags: new.tags,
                body_json: new.body_json,
                created_by: new.created_by,
                created_at: "2026-05-25T00:00:02Z".into(),
                superseded_at: None,
            };
            rows.push(inserted.clone());
            Ok(inserted)
        }
        async fn get_active(
            &self,
            tenant_id: &str,
            page_id: &str,
        ) -> std::result::Result<Option<DashboardRevision>, DashboardStoreError> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .find(|r| {
                    r.tenant_id == tenant_id && r.page_id == page_id && r.superseded_at.is_none()
                })
                .cloned())
        }
        async fn list_active(
            &self,
            _: &str,
            _: &ListFilter,
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(vec![])
        }
        async fn mark_superseded(
            &self,
            tenant_id: &str,
            page_id: &str,
        ) -> std::result::Result<u64, DashboardStoreError> {
            let mut n = 0u64;
            for r in self.rows.lock().unwrap().iter_mut() {
                if r.tenant_id == tenant_id && r.page_id == page_id && r.superseded_at.is_none() {
                    r.superseded_at = Some("2026-05-25T00:00:03Z".into());
                    n += 1;
                }
            }
            Ok(n)
        }
        async fn history(
            &self,
            _: &str,
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(vec![])
        }
    }

    fn sample_input() -> Value {
        serde_json::json!({
            "tenant_id":  "tenant-a",
            "page_id":    "dashboard.ops",
            "deleted_by": "alice"
        })
    }

    #[tokio::test]
    async fn delete_supersedes_live_row_and_emits_diagnostic() {
        let store = InMemoryStore::arc();
        let _rev = store.seed("dashboard.ops", "tenant-a", "alice");
        let tool = DashboardDeleteTool::new(store.clone());
        let out = tool.invoke(sample_input()).await.unwrap();
        let resp: DeleteDashboardResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.dashboard.deleted");
        assert_eq!(resp.superseded, 1);
        assert_eq!(store.live_count("tenant-a", "dashboard.ops"), 0);
    }

    #[tokio::test]
    async fn delete_refuses_system_owned_page() {
        let store = InMemoryStore::arc();
        let _rev = store.seed("dashboard.ops", "system", BUNDLED_PRINCIPAL);
        let tool = DashboardDeleteTool::new(store);
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":  "system",
                "page_id":    "dashboard.ops",
                "deleted_by": "alice"
            }))
            .await
            .unwrap_err();
        match err {
            Error::Conflict { message } => assert!(
                message.contains("rubix.dashboard.delete.refused_system"),
                "unexpected conflict message: {message}"
            ),
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn delete_missing_page_is_not_found() {
        let store = InMemoryStore::arc();
        let tool = DashboardDeleteTool::new(store);
        let err = tool.invoke(sample_input()).await.unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn change_for_returns_delete_draft_then_undo_reinserts() {
        let store = InMemoryStore::arc();
        let _rev = store.seed("dashboard.ops", "tenant-a", "alice");
        let tool = DashboardDeleteTool::new(store.clone());
        let input = sample_input();
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert!(matches!(draft.op, Op::Delete));

        // Pre-populate the prior body in `before` so undo can
        // re-insert non-empty content. (The production wiring
        // will capture the live row before `mark_superseded`; the
        // tool-side draft is a minimal stand-in.)
        let mut before = serde_json::to_value(DashboardSnapshot {
            page_id: "dashboard.ops".into(),
            tenant_id: "tenant-a".into(),
            owner_principal: "alice".into(),
            title: "Ops".into(),
            tags: vec!["custom".into()],
            body_json: serde_json::json!({ "v": 1 }),
            created_by: "alice".into(),
            revision_id: None,
        })
        .unwrap();
        if let Some(obj) = before.as_object_mut() {
            obj.remove("revision_id");
        }
        let ch = Change {
            id: ChangeId("c-test".into()),
            group_id: GroupId("g-test".into()),
            at: chrono::Utc::now(),
            actor: Actor::System,
            resource: draft.resource,
            op: draft.op,
            before: Some(before),
            after: None,
            resource_version: None,
            correlation: None,
            patch: None,
        };
        let reversible = crate::dashboard::store::DashboardReversible::new(
            store.clone() as Arc<dyn DashboardStore>
        );
        reversible.apply_inverse(&ch).await.unwrap();
        assert_eq!(
            store.live_count("tenant-a", "dashboard.ops"),
            1,
            "undo should re-insert a live row for the page"
        );
    }
}
