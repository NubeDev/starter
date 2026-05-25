//! `rubix.dashboard.duplicate` — tool dispatch.
//!
//! Reads the active revision of the source page via
//! [`DashboardStore::get_active`] and inserts a new revision at
//! `target_page_id` whose body matches the source. The store
//! mints a fresh `revision_id`; the new row gets the caller's
//! `new_owner_principal` / `created_by` so the duplicate is
//! operator-owned even when the source was bundled. On a missing
//! source the verb returns a structured [`Diagnostic`] keyed
//! `rubix.dashboard.duplicate.source_not_found`.
//!
//! The [`ReversibleTool`] impl records an `Op::Create`
//! `ChangeDraft` whose `after` payload is a [`DashboardSnapshot`]
//! of the duplicate; the dispatcher walks this back through
//! [`super::store::DashboardReversible`] which soft-deletes the
//! duplicate on undo (same shape as `rubix.dashboard.create`).
//!
//! See `rubix/docs/scope/dashboards/04-tools.md`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::{DashboardStore, NewRevision};
use rubix_spi::dto::dashboard::duplicate::{
    DuplicateDashboardRequest, DuplicateDashboardResponse,
};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::dashboard::store::{DashboardSnapshot, DASHBOARD_PAGE_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.dashboard.duplicate`.
pub struct DashboardDuplicateTool {
    store: Arc<dyn DashboardStore>,
}

impl DashboardDuplicateTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn DashboardStore>) -> Self {
        Self { store }
    }
}

/// Same slug grammar `rubix.dashboard.create` validates against.
fn valid_page_id(id: &str) -> bool {
    let Some(slug) = id.strip_prefix("dashboard.") else {
        return false;
    };
    !slug.is_empty()
        && slug.len() <= 128
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
}

#[async_trait]
impl Tool for DashboardDuplicateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.dashboard.duplicate".to_owned(),
            description: rubix_spi::dto::dashboard::duplicate::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source_tenant_id":    { "type": "string", "minLength": 1 },
                    "source_page_id":      { "type": "string", "minLength": 1 },
                    "target_tenant_id":    { "type": "string", "minLength": 1 },
                    "target_page_id":      { "type": "string", "minLength": 1 },
                    "new_owner_principal": { "type": "string", "minLength": 1 },
                    "created_by":          { "type": "string", "minLength": 1 },
                    "title":               { "type": ["string", "null"] },
                    "tags":                { "type": ["array",  "null"], "items": { "type": "string" } }
                },
                "required": [
                    "source_tenant_id", "source_page_id",
                    "target_tenant_id", "target_page_id",
                    "new_owner_principal", "created_by"
                ],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: DuplicateDashboardRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("DuplicateDashboardRequest: {e}"),
            })?;

        if !valid_page_id(&req.target_page_id) {
            return Err(Error::Invalid {
                message: format!(
                    "target_page_id `{}` must match `dashboard.<lowercase-slug>`",
                    req.target_page_id
                ),
            });
        }

        // Refuse if the target id is already taken — surface the
        // `rubix.dashboard.create.duplicate_id` diagnostic so the
        // consumer can pattern-match on the same conflict key
        // `create` uses.
        if self
            .store
            .get_active(&req.target_tenant_id, &req.target_page_id)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?
            .is_some()
        {
            let diag = Diagnostic::new(
                MessageKey::parse("rubix.dashboard.create.duplicate_id")
                    .expect("hard-coded key parses"),
            )
            .with_param("page_id", DiagnosticParam::String(req.target_page_id.clone()));
            return Err(Error::Conflict {
                message: format!("{}: {}", diag.code.as_str(), req.target_page_id),
            });
        }

        // Source must exist; otherwise surface the dedicated
        // `rubix.dashboard.duplicate.source_not_found` key.
        let source = self
            .store
            .get_active(&req.source_tenant_id, &req.source_page_id)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?
            .ok_or_else(|| {
                let diag = Diagnostic::new(
                    MessageKey::parse("rubix.dashboard.duplicate.source_not_found")
                        .expect("hard-coded key parses"),
                )
                .with_param(
                    "source_page_id",
                    DiagnosticParam::String(req.source_page_id.clone()),
                );
                Error::NotFound {
                    what: format!("{}: {}", diag.code.as_str(), req.source_page_id),
                }
            })?;

        let title = req.title.clone().unwrap_or_else(|| source.title.clone());
        let tags = req.tags.clone().unwrap_or_else(|| source.tags.clone());

        let row = self
            .store
            .insert_revision(NewRevision {
                page_id: req.target_page_id.clone(),
                tenant_id: req.target_tenant_id.clone(),
                owner_principal: req.new_owner_principal.clone(),
                title: title.clone(),
                tags: tags.clone(),
                body_json: source.body_json.clone(),
                created_by: req.created_by.clone(),
            })
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.dashboard.duplicated").expect("hard-coded key parses"),
        )
        .with_param(
            "source_page_id",
            DiagnosticParam::String(req.source_page_id.clone()),
        )
        .with_param("page_id", DiagnosticParam::String(row.page_id.clone()));

        let response = DuplicateDashboardResponse {
            summary,
            source_page_id: req.source_page_id,
            page_id: row.page_id,
            revision_id: row.revision_id,
            tenant_id: row.tenant_id,
            owner_principal: row.owner_principal,
            title: row.title,
            tags: row.tags,
            created_by: row.created_by,
            created_at: row.created_at,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for DashboardDuplicateTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: DuplicateDashboardResponse = serde_json::from_value(output.clone()).ok()?;
        // The duplicate is fresh content — undo retires it via
        // `mark_superseded`, mirroring `dashboard.create`. The
        // snapshot's `body_json` is `Null` here because the
        // response does not echo the cloned body; the undo path
        // for `Op::Create` only needs the tenant/page id pair to
        // find rows to supersede.
        let snap = DashboardSnapshot {
            page_id: resp.page_id.clone(),
            tenant_id: resp.tenant_id.clone(),
            owner_principal: resp.owner_principal.clone(),
            title: resp.title.clone(),
            tags: resp.tags.clone(),
            body_json: Value::Null,
            created_by: resp.created_by.clone(),
            revision_id: Some(resp.revision_id.clone()),
        };
        let after = serde_json::to_value(&snap).ok()?;
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: DASHBOARD_PAGE_KIND.into(),
                id: Some(resp.page_id),
                owner: Some(resp.owner_principal),
                tenant: Some(resp.tenant_id),
            },
            op: Op::Create,
            before: None,
            after: Some(after),
            resource_version: None,
            correlation: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rubix_spi::dashboard::{
        DashboardRevision, DashboardStoreError, ListFilter, NewRevision,
    };
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
        fn seed(&self, page_id: &str, tenant: &str, body: Value) -> String {
            let rev = self.next_rev();
            self.rows.lock().unwrap().push(DashboardRevision {
                page_id: page_id.into(),
                revision_id: rev.clone(),
                tenant_id: tenant.into(),
                owner_principal: "system".into(),
                title: "Disk overview".into(),
                tags: vec!["bundled".into()],
                body_json: body,
                created_by: "system".into(),
                created_at: "2026-05-25T00:00:00Z".into(),
                superseded_at: None,
            });
            rev
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
                    r.tenant_id == tenant_id
                        && r.page_id == page_id
                        && r.superseded_at.is_none()
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
                if r.tenant_id == tenant_id
                    && r.page_id == page_id
                    && r.superseded_at.is_none()
                {
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
            "source_tenant_id":    "system",
            "source_page_id":      "dashboard.disk-overview",
            "target_tenant_id":    "tenant-a",
            "target_page_id":      "dashboard.disk-mine",
            "new_owner_principal": "alice",
            "created_by":          "alice"
        })
    }

    #[tokio::test]
    async fn duplicate_writes_new_row_with_same_body_and_emits_diagnostic() {
        let store = InMemoryStore::arc();
        let body = serde_json::json!({ "ir_version": 1, "root": { "kind": "Stack" } });
        store.seed("dashboard.disk-overview", "system", body.clone());
        let tool = DashboardDuplicateTool::new(store.clone());
        let out = tool.invoke(sample_input()).await.unwrap();
        let resp: DuplicateDashboardResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.dashboard.duplicated");
        assert_eq!(resp.page_id, "dashboard.disk-mine");
        assert_eq!(resp.tenant_id, "tenant-a");
        assert_eq!(resp.owner_principal, "alice");
        // Body propagated to the new row.
        let live = store
            .get_active("tenant-a", "dashboard.disk-mine")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(live.body_json, body);
        // Fresh revision id (rev-1 = source, rev-2 = duplicate).
        assert_ne!(live.revision_id, "rev-1");
    }

    #[tokio::test]
    async fn duplicate_missing_source_returns_not_found_with_diagnostic() {
        let store = InMemoryStore::arc();
        let tool = DashboardDuplicateTool::new(store);
        let err = tool.invoke(sample_input()).await.unwrap_err();
        match err {
            Error::NotFound { what } => assert!(
                what.contains("rubix.dashboard.duplicate.source_not_found"),
                "unexpected not-found payload: {what}"
            ),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn duplicate_target_already_exists_returns_conflict() {
        let store = InMemoryStore::arc();
        store.seed(
            "dashboard.disk-overview",
            "system",
            serde_json::json!({ "v": 1 }),
        );
        store.seed(
            "dashboard.disk-mine",
            "tenant-a",
            serde_json::json!({ "v": 0 }),
        );
        let tool = DashboardDuplicateTool::new(store);
        let err = tool.invoke(sample_input()).await.unwrap_err();
        match err {
            Error::Conflict { message } => assert!(
                message.contains("rubix.dashboard.create.duplicate_id"),
                "unexpected conflict message: {message}"
            ),
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn duplicate_invalid_target_page_id_is_rejected() {
        let store = InMemoryStore::arc();
        store.seed(
            "dashboard.disk-overview",
            "system",
            serde_json::json!({ "v": 1 }),
        );
        let tool = DashboardDuplicateTool::new(store);
        let mut input = sample_input();
        input["target_page_id"] = serde_json::json!("not-a-dashboard-id");
        let err = tool.invoke(input).await.unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn duplicate_then_undo_supersedes_the_duplicate() {
        let store = InMemoryStore::arc();
        store.seed(
            "dashboard.disk-overview",
            "system",
            serde_json::json!({ "v": 1 }),
        );
        let tool = DashboardDuplicateTool::new(store.clone());
        let input = sample_input();
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert!(matches!(draft.op, Op::Create));

        let ch = Change {
            id: ChangeId("c-test".into()),
            group_id: GroupId("g-test".into()),
            at: chrono::Utc::now(),
            actor: Actor::System,
            resource: draft.resource,
            op: draft.op,
            before: draft.before,
            after: draft.after,
            resource_version: None,
            correlation: None,
            patch: None,
        };
        let reversible = crate::dashboard::store::DashboardReversible::new(
            store.clone() as Arc<dyn DashboardStore>,
        );
        reversible.apply_inverse(&ch).await.unwrap();
        assert!(store
            .get_active("tenant-a", "dashboard.disk-mine")
            .await
            .unwrap()
            .is_none());
    }
}
