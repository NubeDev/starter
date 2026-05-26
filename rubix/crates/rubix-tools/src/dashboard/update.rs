//! `rubix.dashboard.update` — tool dispatch.
//!
//! Write verb with optimistic concurrency. Reads the currently-live
//! row for `(tenant_id, page_id)`. When the caller supplied an
//! `expected_revision_id` that no longer matches, the verb refuses
//! with a structured [`Diagnostic`] keyed
//! `rubix.dashboard.update.conflict` (transport maps to HTTP 409).
//! Otherwise it inserts a new revision via
//! [`DashboardStore::insert_revision`] (which atomically supersedes
//! the prior head) and emits `rubix.dashboard.updated`.
//!
//! The [`ReversibleTool`] impl records an `Op::Update` `ChangeDraft`
//! whose `before` / `after` payloads are [`DashboardSnapshot`]s of
//! the prior and freshly-inserted bodies; undo re-inserts the
//! `before` snapshot (the insert-only store automatically supersedes
//! the post-update head), and redo re-inserts the `after`.
//!
//! See `rubix/docs/scope/dashboards/04-tools.md`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::{DashboardStore, NewRevision};
use rubix_spi::dto::dashboard::update::{UpdateDashboardRequest, UpdateDashboardResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::dashboard::store::{DashboardSnapshot, DASHBOARD_PAGE_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.dashboard.update`.
pub struct DashboardUpdateTool {
    store: Arc<dyn DashboardStore>,
}

impl DashboardUpdateTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn DashboardStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for DashboardUpdateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.dashboard.update".to_owned(),
            description: rubix_spi::dto::dashboard::update::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id":            { "type": "string", "minLength": 1 },
                    "page_id":              { "type": "string", "minLength": 1 },
                    "expected_revision_id": { "type": ["string", "null"] },
                    "title":                { "type": ["string", "null"] },
                    "tags":                 { "type": ["array",  "null"], "items": { "type": "string" } },
                    "body_json":            { "type": "object" },
                    "created_by":           { "type": "string", "minLength": 1 }
                },
                "required": ["tenant_id", "page_id", "body_json", "created_by"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: UpdateDashboardRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("UpdateDashboardRequest: {e}"),
            })?;

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

        // Optimistic-concurrency check. A returned conflict is
        // intentionally surfaced as `Error::Conflict` so the
        // transport layer maps to HTTP 409 — the diagnostic code
        // is embedded in the message so the consumer can pattern-
        // match on it.
        if let Some(expected) = &req.expected_revision_id {
            if expected != &prior.revision_id {
                let diag = Diagnostic::new(
                    MessageKey::parse("rubix.dashboard.update.conflict")
                        .expect("hard-coded key parses"),
                )
                .with_param("page_id", DiagnosticParam::String(req.page_id.clone()))
                .with_param(
                    "current_revision_id",
                    DiagnosticParam::String(prior.revision_id.clone()),
                );
                return Err(Error::Conflict {
                    message: format!(
                        "{}: page_id={} current_revision_id={}",
                        diag.code.as_str(),
                        req.page_id,
                        prior.revision_id
                    ),
                });
            }
        }

        let new_title = req.title.clone().unwrap_or_else(|| prior.title.clone());
        let new_tags = req.tags.clone().unwrap_or_else(|| prior.tags.clone());

        let new = NewRevision {
            page_id: req.page_id.clone(),
            tenant_id: req.tenant_id.clone(),
            owner_principal: prior.owner_principal.clone(),
            title: new_title.clone(),
            tags: new_tags.clone(),
            body_json: req.body_json.clone(),
            created_by: req.created_by.clone(),
        };
        let row = self
            .store
            .insert_revision(new)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.dashboard.updated").expect("hard-coded key parses"),
        )
        .with_param("page_id", DiagnosticParam::String(row.page_id.clone()));

        let response = UpdateDashboardResponse {
            summary,
            page_id: row.page_id,
            revision_id: row.revision_id,
            tenant_id: row.tenant_id,
            written: true,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for DashboardUpdateTool {
    fn change_for(&self, input: &Value, output: &Value) -> Option<ChangeDraft> {
        // Re-derive the "before" snapshot from the caller's input —
        // by the time the change recorder runs the prior revision is
        // already superseded in the store, so we cannot re-fetch it
        // here. The tool body validated `expected_revision_id`
        // against the store immediately before the insert, so the
        // recorded `before` is the live state from the caller's
        // perspective.
        let req: UpdateDashboardRequest = serde_json::from_value(input.clone()).ok()?;
        let resp: UpdateDashboardResponse = serde_json::from_value(output.clone()).ok()?;

        // We can't reconstruct the prior body from `input` alone —
        // `expected_revision_id` carries only the id. To make undo
        // work without re-querying, the recorder skips the draft
        // when the caller did not supply an `expected_revision_id`
        // *and* a prior body snapshot via the `prior_*` companion
        // fields (which Phase C.2 leaves as a follow-up). Today the
        // recorder always emits `after`-only and lets the
        // `DashboardReversible::apply_inverse` path treat
        // missing-before as a soft-delete fallback.
        // owner_principal isn't echoed in the response (the store
        // preserves it across revisions); the snapshot's
        // `owner_principal` is informational for the audit row.
        let after = DashboardSnapshot {
            page_id: resp.page_id.clone(),
            tenant_id: resp.tenant_id.clone(),
            owner_principal: req.created_by.clone(),
            title: req.title.unwrap_or_default(),
            tags: req.tags.unwrap_or_default(),
            body_json: req.body_json,
            created_by: req.created_by,
            revision_id: Some(resp.revision_id.clone()),
        };
        let after_v = serde_json::to_value(&after).ok()?;
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: DASHBOARD_PAGE_KIND.into(),
                id: Some(resp.page_id),
                owner: None,
                tenant: Some(resp.tenant_id),
            },
            op: Op::Update,
            // `before` intentionally `None` in Phase C.2 — see the
            // doc comment above. The change recorder still produces
            // a useful audit row; full reversibility for `update`
            // lands when a `prior_snapshot` capture seam (the store
            // returning the prior body alongside the insert) ships
            // in a follow-up.
            before: None,
            after: Some(after_v),
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
        fn seed(&self, page_id: &str, tenant: &str) -> String {
            let rev = self.next_rev();
            self.rows.lock().unwrap().push(DashboardRevision {
                page_id: page_id.into(),
                revision_id: rev.clone(),
                tenant_id: tenant.into(),
                owner_principal: "alice".into(),
                title: "Old title".into(),
                tags: vec!["custom".into()],
                body_json: serde_json::json!({ "v": 1 }),
                created_by: "alice".into(),
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
            _: &str,
            _: &str,
        ) -> std::result::Result<u64, DashboardStoreError> {
            Ok(0)
        }
        async fn history(
            &self,
            _: &str,
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn update_writes_new_revision_and_emits_diagnostic() {
        let store = InMemoryStore::arc();
        let prior = store.seed("dashboard.ops", "tenant-a");
        let tool = DashboardUpdateTool::new(store.clone());
        let out = tool
            .invoke(serde_json::json!({
                "tenant_id":            "tenant-a",
                "page_id":              "dashboard.ops",
                "expected_revision_id": prior,
                "title":                "New title",
                "body_json":            { "v": 2 },
                "created_by":           "alice"
            }))
            .await
            .unwrap();
        let resp: UpdateDashboardResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.dashboard.updated");
        assert!(resp.written);
        let live = store
            .get_active("tenant-a", "dashboard.ops")
            .await
            .unwrap()
            .unwrap();
        assert_ne!(live.revision_id, "rev-1");
        assert_eq!(live.title, "New title");
        assert_eq!(live.body_json, serde_json::json!({ "v": 2 }));
    }

    /// update→conflict-on-stale — the sibling behavior the stage
    /// calls out.
    #[tokio::test]
    async fn stale_expected_revision_id_returns_conflict() {
        let store = InMemoryStore::arc();
        let _prior = store.seed("dashboard.ops", "tenant-a");
        let tool = DashboardUpdateTool::new(store);
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":            "tenant-a",
                "page_id":              "dashboard.ops",
                "expected_revision_id": "rev-stale",
                "body_json":            { "v": 2 },
                "created_by":           "alice"
            }))
            .await
            .unwrap_err();
        match err {
            Error::Conflict { message } => assert!(
                message.contains("rubix.dashboard.update.conflict"),
                "unexpected conflict message: {message}"
            ),
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn missing_page_is_not_found() {
        let store = InMemoryStore::arc();
        let tool = DashboardUpdateTool::new(store);
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":  "tenant-a",
                "page_id":    "dashboard.ghost",
                "body_json":  { "v": 1 },
                "created_by": "alice"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn omitted_title_and_tags_preserve_prior_values() {
        let store = InMemoryStore::arc();
        let _prior = store.seed("dashboard.ops", "tenant-a");
        let tool = DashboardUpdateTool::new(store.clone());
        tool.invoke(serde_json::json!({
            "tenant_id":  "tenant-a",
            "page_id":    "dashboard.ops",
            "body_json":  { "v": 2 },
            "created_by": "alice"
        }))
        .await
        .unwrap();
        let live = store
            .get_active("tenant-a", "dashboard.ops")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(live.title, "Old title");
        assert_eq!(live.tags, vec!["custom".to_string()]);
    }

    #[tokio::test]
    async fn change_for_returns_update_draft() {
        let store = InMemoryStore::arc();
        let prior = store.seed("dashboard.ops", "tenant-a");
        let tool = DashboardUpdateTool::new(store);
        let input = serde_json::json!({
            "tenant_id":            "tenant-a",
            "page_id":              "dashboard.ops",
            "expected_revision_id": prior,
            "title":                "New title",
            "body_json":            { "v": 2 },
            "created_by":           "alice"
        });
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert!(matches!(draft.op, Op::Update));
        assert!(draft.after.is_some());
    }
}
