//! `rubix.dashboard.create` — tool dispatch.
//!
//! Write verb. Validates the `page_id` shape, refuses a duplicate
//! id with a structured [`Diagnostic`] keyed
//! `rubix.dashboard.create.duplicate_id`, inserts the first
//! revision through [`DashboardStore::insert_revision`], and
//! re-asserts the `rubix.dashboard.page` `ResourceSpec` on the
//! authz registry (idempotent — a `DuplicateResource` is treated as
//! success). The [`ReversibleTool`] impl records an `Op::Create`
//! `ChangeDraft` whose `after` payload is a [`DashboardSnapshot`];
//! the dispatcher walks this back through
//! [`super::store::DashboardReversible`] which soft-deletes the
//! page on undo.
//!
//! See `rubix/docs/scope/dashboards/04-tools.md`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::{DashboardStore, NewRevision};
use rubix_spi::dto::dashboard::create::{CreateDashboardRequest, CreateDashboardResponse};
use serde_json::Value;
use starter_authz::error::Error as AuthzError;
use starter_authz::StaticRegistry;
use starter_spi::authz::{Ownership, ResourceRef, ResourceSpec};
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::dashboard::store::{DashboardSnapshot, DASHBOARD_PAGE_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.dashboard.create`.
pub struct DashboardCreateTool {
    store: Arc<dyn DashboardStore>,
    registry: Arc<StaticRegistry>,
}

impl DashboardCreateTool {
    /// Wrap the shared store + authz registry.
    pub fn new(store: Arc<dyn DashboardStore>, registry: Arc<StaticRegistry>) -> Self {
        Self { store, registry }
    }
}

/// Validate `body_json` against the `ComponentTree` schema.
///
/// The store accepts any `serde_json::Value` (the column is `jsonb`),
/// so without this gate the writer happily persists garbage shapes
/// that only fail later in the SDUI resolver — producing 404s at
/// page-load time with no signal at create time. Validating here
/// fails the tool call with `Invalid` so the caller (CLI, MCP,
/// chat agent) sees the schema error immediately.
fn validate_body_json(body: &Value) -> Result<()> {
    match serde_json::from_value::<starter_ui_ir::ComponentTree>(body.clone()) {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::Invalid {
            message: format!(
                "body_json does not match the `ComponentTree` schema: {e}. \
                 Expected `{{\"ir_version\":1,\"root\":{{\"type\":\"page\",...}}}}` \
                 — see `starter-ui-ir` for the full shape.",
            ),
        }),
    }
}

/// Build (and re-register) the `rubix.dashboard.page` `ResourceSpec`.
/// `DuplicateResource` is treated as success — the kind is also
/// registered at boot, this call merely ensures it on every write.
fn ensure_resource_kind(registry: &StaticRegistry) {
    let spec = ResourceSpec::from_static_tenant_scoped(
        DASHBOARD_PAGE_KIND,
        &["view", "edit", "delete"],
        Ownership::Subject,
        "Rubix dashboard page",
        "An SDUI page persisted in `dashboards_definitions` and resolved by the page provider.",
    );
    match registry.try_register(spec) {
        Ok(()) | Err(AuthzError::DuplicateResource { .. }) => {}
        Err(other) => tracing::warn!(
            target: "rubix.dashboard.create",
            error = %other,
            "ResourceSpec re-register failed",
        ),
    }
}

#[async_trait]
impl Tool for DashboardCreateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.dashboard.create".to_owned(),
            description: rubix_spi::dto::dashboard::create::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id":       { "type": "string", "minLength": 1 },
                    "page_id":         { "type": "string", "minLength": 1 },
                    "owner_principal": { "type": "string", "minLength": 1 },
                    "title":           { "type": "string", "minLength": 1 },
                    "tags":            { "type": "array",  "items": { "type": "string" } },
                    "body_json":       { "type": "object" },
                    "created_by":      { "type": "string", "minLength": 1 }
                },
                "required": [
                    "tenant_id", "page_id", "owner_principal", "title",
                    "body_json", "created_by"
                ],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: CreateDashboardRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("CreateDashboardRequest: {e}"),
            })?;

        if let Err(e) = crate::dashboard::page_id::validate_stored_page_id(&req.page_id) {
            return Err(Error::Invalid {
                message: e.message("page_id"),
            });
        }

        validate_body_json(&req.body_json)?;
        crate::dashboard::layout::validate_layout(&req.body_json)?;

        // Duplicate-id probe. Surfaces a structured diagnostic via
        // Error::Conflict so the transport layer maps to HTTP 409.
        let existing = self
            .store
            .get_active(&req.tenant_id, &req.page_id)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;
        if existing.is_some() {
            let diag = Diagnostic::new(
                MessageKey::parse("rubix.dashboard.create.duplicate_id")
                    .expect("hard-coded key parses"),
            )
            .with_param("page_id", DiagnosticParam::String(req.page_id.clone()));
            return Err(Error::Conflict {
                message: format!("{}: {}", diag.code.as_str(), req.page_id),
            });
        }

        let new = NewRevision {
            page_id: req.page_id.clone(),
            tenant_id: req.tenant_id.clone(),
            owner_principal: req.owner_principal.clone(),
            title: req.title.clone(),
            tags: req.tags.clone(),
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

        // Re-assert the per-tenant resource kind. Idempotent.
        ensure_resource_kind(&self.registry);

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.dashboard.created").expect("hard-coded key parses"),
        )
        .with_param("title", DiagnosticParam::String(row.title.clone()))
        .with_param("page_id", DiagnosticParam::String(row.page_id.clone()));

        let response = CreateDashboardResponse {
            summary,
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

impl ReversibleTool for DashboardCreateTool {
    fn change_for(&self, input: &Value, output: &Value) -> Option<ChangeDraft> {
        let req: CreateDashboardRequest = serde_json::from_value(input.clone()).ok()?;
        let resp: CreateDashboardResponse = serde_json::from_value(output.clone()).ok()?;
        let snap = DashboardSnapshot {
            page_id: resp.page_id.clone(),
            tenant_id: resp.tenant_id.clone(),
            owner_principal: resp.owner_principal.clone(),
            title: resp.title.clone(),
            tags: resp.tags.clone(),
            body_json: req.body_json,
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
    use rubix_spi::dashboard::{DashboardRevision, DashboardStoreError, ListFilter, NewRevision};
    use starter_spi::authz::ResourceRegistry;
    use starter_spi::changelog::{Actor, Change, ChangeId, GroupId, Reversible};
    use std::sync::Mutex;

    /// In-memory store mirroring the PG insert-only contract.
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
        fn live_count(&self) -> usize {
            self.rows
                .lock()
                .unwrap()
                .iter()
                .filter(|r| r.superseded_at.is_none())
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
                created_at: "2026-05-25T00:00:00Z".into(),
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
                    r.superseded_at = Some("2026-05-25T00:00:02Z".into());
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
            "tenant_id":       "tenant-a",
            "page_id":         "dashboard.ops",
            "owner_principal": "alice",
            "title":           "Ops",
            "tags":            ["custom"],
            "body_json":       {
                "ir_version": 1,
                "root": { "type": "page", "id": "p", "children": [] }
            },
            "created_by":      "alice"
        })
    }

    #[tokio::test]
    async fn create_emits_diagnostic_and_persists_row() {
        let store = InMemoryStore::arc();
        let registry = Arc::new(StaticRegistry::new());
        let tool = DashboardCreateTool::new(store.clone(), registry.clone());
        let out = tool.invoke(sample_input()).await.unwrap();
        let resp: CreateDashboardResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.dashboard.created");
        assert_eq!(resp.page_id, "dashboard.ops");
        assert_eq!(resp.revision_id, "rev-1");
        assert_eq!(store.live_count(), 1);
        // Registry now knows the kind.
        assert!(registry.lookup(DASHBOARD_PAGE_KIND).is_some());
    }

    #[tokio::test]
    async fn duplicate_page_id_refused_with_conflict() {
        let store = InMemoryStore::arc();
        let registry = Arc::new(StaticRegistry::new());
        let tool = DashboardCreateTool::new(store.clone(), registry);
        tool.invoke(sample_input()).await.unwrap();
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
    async fn invalid_page_id_is_rejected() {
        let store = InMemoryStore::arc();
        let registry = Arc::new(StaticRegistry::new());
        let tool = DashboardCreateTool::new(store, registry);
        let mut input = sample_input();
        input["page_id"] = serde_json::json!("not-a-dashboard-id");
        let err = tool.invoke(input).await.unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn bare_slug_diagnostic_suggests_stored_form() {
        // Issue #5: a caller passing the URL-form `ops` instead of
        // the stored-form `dashboard.ops` should get a message that
        // names both forms and the corrected id, not the previous
        // generic "must match dashboard.<lowercase-slug>" error.
        let store = InMemoryStore::arc();
        let registry = Arc::new(StaticRegistry::new());
        let tool = DashboardCreateTool::new(store, registry);
        let mut input = sample_input();
        input["page_id"] = serde_json::json!("ops");
        let err = tool.invoke(input).await.unwrap_err();
        match err {
            Error::Invalid { message } => {
                assert!(
                    message.contains("dashboard.ops"),
                    "expected message to suggest stored form, got: {message}"
                );
                assert!(
                    message.contains("URL"),
                    "expected message to mention URL form, got: {message}"
                );
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    /// create→undo→deleted (i.e. soft-superseded) — the sibling
    /// behavior the stage calls out.
    #[tokio::test]
    async fn create_then_undo_supersedes_the_page() {
        let store = InMemoryStore::arc();
        let registry = Arc::new(StaticRegistry::new());
        let tool = DashboardCreateTool::new(store.clone(), registry);
        let input = sample_input();
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert!(matches!(draft.op, Op::Create));

        let reversible = crate::dashboard::store::DashboardReversible::new(
            store.clone() as Arc<dyn DashboardStore>
        );
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
        reversible.apply_inverse(&ch).await.unwrap();
        assert_eq!(
            store.live_count(),
            0,
            "undo should supersede every live row for the page"
        );
    }
}
