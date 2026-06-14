//! Store traits (DOCS §4). Backends in `starter-store-sqlite` /
//! `-postgres` implement these behind a `setup` feature, exactly like the
//! flow stores.

use async_trait::async_trait;
use starter_flow_spi::flow::RunId;

use crate::error::SetupResult;
use crate::model::{
    Progress, SemVer, SetupRun, SetupRunStatus, Template, TemplateId, TemplateSummary,
};

/// Reserved `tenant_id` sentinel namespacing extension-provided templates
/// that all tenants inherit (DOCS §5). A same-`(id,version)` row under a
/// real tenant overrides the global one (the read path prefers tenant
/// rows over global).
pub const GLOBAL_TENANT_SENTINEL: &str = "__global__";

/// Filter for [`TemplateStore::list`].
#[derive(Debug, Clone, Default)]
pub struct TemplateFilter {
    /// Restrict to a tenant. The read path additionally folds in
    /// [`GLOBAL_TENANT_SENTINEL`] rows the tenant inherits.
    pub tenant_id: Option<String>,
    /// Restrict to a nav category.
    pub category: Option<String>,
}

/// Filter for [`SetupRunStore::list`].
#[derive(Debug, Clone, Default)]
pub struct SetupRunFilter {
    /// Restrict to runs launched by this `Principal.subject`.
    pub owner: Option<String>,
    /// Restrict to a tenant.
    pub tenant_id: Option<String>,
    /// Restrict to a template.
    pub template_id: Option<TemplateId>,
    /// Restrict to a status.
    pub status: Option<SetupRunStatus>,
}

/// The template catalog. PK is `(tenant_id, id, version)` with the
/// `__global__` sentinel — never `(id, version)` (DOCS §5).
#[async_trait]
pub trait TemplateStore: Send + Sync + 'static {
    /// Insert or replace a template (keyed by tenant+id+version).
    async fn put(&self, template: Template) -> SetupResult<TemplateId>;

    /// Fetch one template. With `version = None`, returns the latest
    /// version. The read path prefers a tenant row over a `__global__`
    /// row of the same `(id, version)`.
    async fn get(
        &self,
        tenant_id: Option<&str>,
        id: &TemplateId,
        version: Option<SemVer>,
    ) -> SetupResult<Option<Template>>;

    /// List template summaries matching `filter` (tenant rows plus the
    /// inherited global catalog).
    async fn list(&self, filter: TemplateFilter) -> SetupResult<Vec<TemplateSummary>>;

    /// Delete one `(tenant, id, version)`.
    async fn delete(
        &self,
        tenant_id: Option<&str>,
        id: &TemplateId,
        version: SemVer,
    ) -> SetupResult<()>;
}

/// The run index — a thin catalog over flow `RunId`s.
#[async_trait]
pub trait SetupRunStore: Send + Sync + 'static {
    /// Record a freshly launched run.
    async fn record(&self, run: SetupRun) -> SetupResult<()>;

    /// Fetch one run by id.
    async fn get(&self, run_id: RunId) -> SetupResult<Option<SetupRun>>;

    /// List runs matching `filter`.
    async fn list(&self, filter: SetupRunFilter) -> SetupResult<Vec<SetupRun>>;

    /// Update the progress/status projection for a run (called by the
    /// progress projector as `FlowEvent`s arrive).
    async fn update_progress(
        &self,
        run_id: RunId,
        progress: Progress,
        status: SetupRunStatus,
    ) -> SetupResult<()>;

    /// Mark a run terminal-failed with a resume cursor (DOCS §8b).
    async fn mark_failed(
        &self,
        run_id: RunId,
        failed_node: Option<String>,
        resumable: bool,
    ) -> SetupResult<()>;

    /// Mark a run terminal-finished (Completed/Cancelled) with the
    /// finished-at timestamp.
    async fn mark_finished(
        &self,
        run_id: RunId,
        status: SetupRunStatus,
        finished_at: String,
    ) -> SetupResult<()>;

    /// Run ids that are not in a terminal *or* are resumable-failed —
    /// the set the boot resumer considers (DOCS §8a crash recovery +
    /// §8b auto-recovery). Returns `Pending`/`Running` plus
    /// `Failed && resumable` rows.
    async fn list_open(&self) -> SetupResult<Vec<RunId>>;
}
