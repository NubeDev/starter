//! `rubix.dashboard.patch` — tool dispatch.
//!
//! Partial-update verb. Applies an RFC 6902 JSON-Patch document to
//! the live `body_json` for `(tenant_id, page_id)` and routes the
//! synthesised body through `DashboardStore::insert_revision` — the
//! *exact* same write path `rubix.dashboard.update` uses. The
//! changelog therefore continues to record a full before/after
//! snapshot for `Op::Update`, and undo round-trips byte-for-byte
//! without having to invert the patch (which would be a class of
//! silent-drift bugs we choose not to inherit).
//!
//! Concurrency contract: when the caller supplies an
//! `expected_revision_id` that no longer matches the live revision,
//! the verb refuses with [`Error::Conflict`] carrying the
//! `rubix.dashboard.patch.conflict` diagnostic key. A malformed
//! patch (bad JSON shape, unknown `op`, dangling `path`, failed
//! `test` op) yields [`Error::Invalid`] keyed
//! `rubix.dashboard.patch.invalid`. On success the verb emits
//! `rubix.dashboard.patched`.
//!
//! See `rubix/docs/scope/dashboards/04-tools.md` and
//! `rubix/docs/design/sdui/dashboard-api-usage.md` issue #4.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::{DashboardStore, NewRevision};
use rubix_spi::dto::dashboard::patch::{PatchDashboardRequest, PatchDashboardResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::dashboard::store::{DashboardSnapshot, DASHBOARD_PAGE_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.dashboard.patch`.
pub struct DashboardPatchTool {
    store: Arc<dyn DashboardStore>,
}

impl DashboardPatchTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn DashboardStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for DashboardPatchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.dashboard.patch".to_owned(),
            description: rubix_spi::dto::dashboard::patch::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id":            { "type": "string", "minLength": 1 },
                    "page_id":              { "type": "string", "minLength": 1 },
                    "expected_revision_id": { "type": ["string", "null"] },
                    "patch":                { "type": "array" },
                    "created_by":           { "type": "string", "minLength": 1 }
                },
                "required": ["tenant_id", "page_id", "patch", "created_by"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: PatchDashboardRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("PatchDashboardRequest: {e}"),
            })?;

        // Parse the RFC 6902 document up-front so a structurally
        // malformed patch fails fast — before we touch the store or
        // burn a revision id.
        let patch: json_patch::Patch =
            serde_json::from_value(req.patch.clone()).map_err(|e| Error::Invalid {
                message: format!("rubix.dashboard.patch.invalid: {e}"),
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

        if let Some(expected) = &req.expected_revision_id {
            if expected != &prior.revision_id {
                let diag = Diagnostic::new(
                    MessageKey::parse("rubix.dashboard.patch.conflict")
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

        // Apply the patch to a clone of the prior body. `json_patch`
        // mutates in-place on success and leaves the input
        // unspecified on failure, so we always start from a fresh
        // clone to avoid leaking partial state into the snapshot
        // path if a later op fails.
        let mut next_body = prior.body_json.clone();
        json_patch::patch(&mut next_body, &patch).map_err(|e| Error::Invalid {
            message: format!("rubix.dashboard.patch.invalid: {e}"),
        })?;

        // Re-validate post-patch: a syntactically-valid patch can
        // still produce a body that violates the page→row→col→widget
        // layout invariants, and we'd rather refuse here than let the
        // SDUI resolver blow up at render time.
        crate::dashboard::layout::validate_layout(&next_body)?;

        let new = NewRevision {
            page_id: req.page_id.clone(),
            tenant_id: req.tenant_id.clone(),
            owner_principal: prior.owner_principal.clone(),
            // Patch is body-only — metadata is preserved across the
            // revision boundary. A caller who wants to rename the
            // page should use `rubix.dashboard.update`.
            title: prior.title.clone(),
            tags: prior.tags.clone(),
            body_json: next_body.clone(),
            created_by: req.created_by.clone(),
        };
        let outcome = self
            .store
            .insert_revision_with_prior(new)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;
        let row = outcome.inserted;
        let prior_body = outcome.prior.map(|p| p.body_json);

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.dashboard.patched").expect("hard-coded key parses"),
        )
        .with_param("page_id", DiagnosticParam::String(row.page_id.clone()));

        let response = PatchDashboardResponse {
            summary,
            page_id: row.page_id,
            revision_id: row.revision_id,
            tenant_id: row.tenant_id,
            written: true,
            // Carried so `change_for` can record a byte-exact
            // `after` snapshot without re-fetching the row.
            body_json: Some(row.body_json),
            // Paired with `body_json`, gives the recorder a
            // byte-exact `before` for the changelog row.
            prior_body_json: prior_body,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for DashboardPatchTool {
    fn change_for(&self, input: &Value, output: &Value) -> Option<ChangeDraft> {
        // Same Phase C.2 limitation as `DashboardUpdateTool`: by the
        // time `change_for` runs, the prior revision is already
        // superseded and we cannot re-fetch it, so the recorded
        // `before` snapshot is `None` and undo for `patch` is
        // best-effort until a `prior_snapshot` capture seam ships.
        // Routing through the same `Op::Update` keeps the changelog
        // wire shape identical to `update` — downstream consumers
        // (SSE emitter, undo stack) do not need a new variant.
        let req: PatchDashboardRequest = serde_json::from_value(input.clone()).ok()?;
        let resp: PatchDashboardResponse = serde_json::from_value(output.clone()).ok()?;

        // Both response fields are populated atomically by the
        // chokepoint: `body_json` is the post-patch body that
        // landed, `prior_body_json` is the superseded body. Title
        // and tags are preserved across the patch boundary
        // (patch never touches metadata), so the snapshot's
        // metadata fields are empty by design — the inverse path
        // applies `body_json` and leaves the row's stored title /
        // tags alone.
        let after = DashboardSnapshot {
            page_id: resp.page_id.clone(),
            tenant_id: resp.tenant_id.clone(),
            owner_principal: req.created_by.clone(),
            title: String::new(),
            tags: Vec::new(),
            body_json: resp.body_json.unwrap_or(Value::Null),
            created_by: req.created_by.clone(),
            revision_id: Some(resp.revision_id.clone()),
        };
        let after_v = serde_json::to_value(&after).ok()?;
        let before_v = resp.prior_body_json.as_ref().and_then(|body| {
            let before = DashboardSnapshot {
                page_id: resp.page_id.clone(),
                tenant_id: resp.tenant_id.clone(),
                owner_principal: req.created_by.clone(),
                title: String::new(),
                tags: Vec::new(),
                body_json: body.clone(),
                created_by: req.created_by.clone(),
                revision_id: req.expected_revision_id.clone(),
            };
            serde_json::to_value(&before).ok()
        });
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: DASHBOARD_PAGE_KIND.into(),
                id: Some(resp.page_id),
                owner: None,
                tenant: Some(resp.tenant_id),
            },
            op: Op::Update,
            before: before_v,
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

    /// A minimal `body_json` that satisfies `validate_layout`. The
    /// patch tests mutate `title` / `children` off this root.
    fn seed_body() -> Value {
        serde_json::json!({
            "ir_version": 5,
            "root": { "type": "page", "id": "p", "children": [] }
        })
    }

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
                owner_principal: "alice".into(),
                title: "Old title".into(),
                tags: vec!["custom".into()],
                body_json: body,
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
    async fn patch_applies_and_writes_new_revision() {
        let store = InMemoryStore::arc();
        let prior = store.seed("dashboard.ops", "tenant-a", seed_body());
        let tool = DashboardPatchTool::new(store.clone());
        let out = tool
            .invoke(serde_json::json!({
                "tenant_id":            "tenant-a",
                "page_id":              "dashboard.ops",
                "expected_revision_id": prior,
                "patch":                [
                    { "op": "add", "path": "/root/title", "value": "Ops (live)" }
                ],
                "created_by":           "alice"
            }))
            .await
            .unwrap();
        let resp: PatchDashboardResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.dashboard.patched");
        assert!(resp.written);
        let live = store
            .get_active("tenant-a", "dashboard.ops")
            .await
            .unwrap()
            .unwrap();
        assert_ne!(live.revision_id, "rev-1");
        // Body was patched.
        assert_eq!(
            live.body_json.pointer("/root/title"),
            Some(&Value::String("Ops (live)".into()))
        );
        // Metadata preserved across the patch revision.
        assert_eq!(live.title, "Old title");
        assert_eq!(live.tags, vec!["custom".to_string()]);
    }

    #[tokio::test]
    async fn stale_expected_revision_id_returns_conflict() {
        let store = InMemoryStore::arc();
        let _prior = store.seed("dashboard.ops", "tenant-a", seed_body());
        let tool = DashboardPatchTool::new(store);
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":            "tenant-a",
                "page_id":              "dashboard.ops",
                "expected_revision_id": "rev-stale",
                "patch":                [
                    { "op": "replace", "path": "/root/title", "value": "x" }
                ],
                "created_by":           "alice"
            }))
            .await
            .unwrap_err();
        match err {
            Error::Conflict { message } => assert!(
                message.contains("rubix.dashboard.patch.conflict"),
                "unexpected conflict message: {message}"
            ),
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn missing_page_is_not_found() {
        let store = InMemoryStore::arc();
        let tool = DashboardPatchTool::new(store);
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":  "tenant-a",
                "page_id":    "dashboard.ghost",
                "patch":      [ { "op": "replace", "path": "/x", "value": 1 } ],
                "created_by": "alice"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn malformed_patch_is_invalid() {
        let store = InMemoryStore::arc();
        let _prior = store.seed("dashboard.ops", "tenant-a", seed_body());
        let tool = DashboardPatchTool::new(store);
        // `op: "frobnicate"` is not a valid RFC 6902 op.
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":  "tenant-a",
                "page_id":    "dashboard.ops",
                "patch":      [ { "op": "frobnicate", "path": "/root", "value": 1 } ],
                "created_by": "alice"
            }))
            .await
            .unwrap_err();
        match err {
            Error::Invalid { message } => assert!(
                message.contains("rubix.dashboard.patch.invalid"),
                "expected invalid diag, got: {message}"
            ),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn patch_targeting_missing_path_is_invalid() {
        let store = InMemoryStore::arc();
        let _prior = store.seed("dashboard.ops", "tenant-a", seed_body());
        let tool = DashboardPatchTool::new(store);
        // `replace` on a path that does not exist is an RFC 6902
        // runtime error and must surface as Invalid.
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":  "tenant-a",
                "page_id":    "dashboard.ops",
                "patch":      [
                    { "op": "replace", "path": "/root/nope/deep", "value": 1 }
                ],
                "created_by": "alice"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }), "got {err:?}");
    }

    #[tokio::test]
    async fn patch_that_breaks_layout_is_rejected() {
        let store = InMemoryStore::arc();
        let _prior = store.seed("dashboard.ops", "tenant-a", seed_body());
        let tool = DashboardPatchTool::new(store);
        // Syntactically valid patch, but the result is no longer a
        // `page` root and so fails layout validation.
        let err = tool
            .invoke(serde_json::json!({
                "tenant_id":  "tenant-a",
                "page_id":    "dashboard.ops",
                "patch":      [
                    { "op": "replace", "path": "/root/type", "value": "row" }
                ],
                "created_by": "alice"
            }))
            .await
            .unwrap_err();
        // validate_layout returns Error::Invalid for non-page roots.
        assert!(matches!(err, Error::Invalid { .. }), "got {err:?}");
    }

    #[tokio::test]
    async fn change_for_records_byte_exact_before_from_prior_row() {
        // The chokepoint variant `insert_revision_with_prior`
        // captures the superseded body atomically; patch's
        // `change_for` records it as the `before` snapshot.
        let store = InMemoryStore::arc();
        let prior_rev = store.seed("dashboard.ops", "tenant-a", seed_body());
        let tool = DashboardPatchTool::new(store);
        let input = serde_json::json!({
            "tenant_id":            "tenant-a",
            "page_id":              "dashboard.ops",
            "expected_revision_id": prior_rev,
            "patch":                [
                { "op": "add", "path": "/root/title", "value": "x" }
            ],
            "created_by":           "alice"
        });
        let output = tool.invoke(input.clone()).await.unwrap();
        let resp: PatchDashboardResponse = serde_json::from_value(output.clone()).unwrap();
        assert_eq!(resp.prior_body_json.as_ref(), Some(&seed_body()));

        let draft = tool.change_for(&input, &output).expect("draft present");
        let before = draft.before.expect("before present");
        assert_eq!(before.get("body_json"), Some(&seed_body()));
    }

    #[tokio::test]
    async fn change_for_after_snapshot_is_byte_exact_post_patch_body() {
        // The Phase C.2 caveat on `update` records an empty `after`
        // body because `UpdateDashboardResponse` doesn't echo it.
        // `patch` carries the post-patch body in its response, so
        // the audit snapshot can be reconstructed exactly.
        let store = InMemoryStore::arc();
        let prior = store.seed("dashboard.ops", "tenant-a", seed_body());
        let tool = DashboardPatchTool::new(store);
        let input = serde_json::json!({
            "tenant_id":            "tenant-a",
            "page_id":              "dashboard.ops",
            "expected_revision_id": prior,
            "patch":                [
                { "op": "add", "path": "/root/title", "value": "Ops (live)" }
            ],
            "created_by":           "alice"
        });
        let output = tool.invoke(input.clone()).await.unwrap();
        let resp: PatchDashboardResponse = serde_json::from_value(output.clone()).unwrap();
        let expected_body = serde_json::json!({
            "ir_version": 5,
            "root": { "type": "page", "id": "p", "children": [], "title": "Ops (live)" }
        });
        assert_eq!(resp.body_json.as_ref(), Some(&expected_body));

        let draft = tool.change_for(&input, &output).expect("draft present");
        let after = draft.after.expect("after present");
        // The snapshot's body_json round-trips the post-patch body.
        assert_eq!(after.get("body_json"), Some(&expected_body));
    }

    #[tokio::test]
    async fn change_for_returns_update_draft() {
        let store = InMemoryStore::arc();
        let prior = store.seed("dashboard.ops", "tenant-a", seed_body());
        let tool = DashboardPatchTool::new(store);
        let input = serde_json::json!({
            "tenant_id":            "tenant-a",
            "page_id":              "dashboard.ops",
            "expected_revision_id": prior,
            "patch":                [
                { "op": "add", "path": "/root/title", "value": "x" }
            ],
            "created_by":           "alice"
        });
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        // Patch routes through Op::Update — downstream changelog
        // consumers see one revision shape for body edits.
        assert!(matches!(draft.op, Op::Update));
        assert!(draft.after.is_some());
    }
}
