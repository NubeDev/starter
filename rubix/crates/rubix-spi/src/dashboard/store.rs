//! `DashboardStore` async trait + value types.
//!
//! Backed by [`rubix-store-postgres::dashboards::PgDashboardStore`]
//! in production; the trait surface is the seam every
//! `rubix.dashboard.*` tool body dispatches through so tests can
//! swap in an in-memory fake without touching SQL.
//!
//! Contract per `rubix/docs/scope/dashboards/01-storage.md`:
//!
//! - **Insert-only writes.** `insert_revision` supersedes any prior
//!   live row for the same `(tenant_id, page_id)` in the same
//!   transaction so the active-set query never sees two heads.
//! - **Active = `superseded_at IS NULL`.** Reads filter on the
//!   active partial index.
//! - **Page IDs are stable across revisions.** A revision id rolls
//!   over per write; the page id does not.

use serde::{Deserialize, Serialize};

/// Sentinel `tenant_id` used by bundled (system-seeded) pages.
/// Mirrors the all-zero UUID the `flows_definitions` seed path
/// uses, only rendered as TEXT here because the dashboard table's
/// `tenant_id` column is TEXT (page ids are TEXT, principals are
/// TEXT — keeping the column stringly-typed lets ad-hoc operators
/// share the same row shape).
pub const BUNDLED_TENANT: &str = "system";

/// Sentinel `owner_principal` / `created_by` used by bundled pages.
/// The `rubix.dashboard.update` / `delete` tool bodies refuse any
/// write whose target row carries this principal (Phase A.4).
pub const BUNDLED_PRINCIPAL: &str = "system";

/// One row from `dashboards_definitions`. `body_json` is the
/// resolved `starter_ui_ir::ComponentTree`; this crate keeps it as
/// [`serde_json::Value`] so `rubix-spi` retains zero deps on the
/// IR crate (validation happens at the tool body, see
/// `04-tools.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardRevision {
    /// Stable SDUI page id, e.g. `"dashboard.disk-overview"`.
    pub page_id: String,
    /// UUID rendered as text — round-trips through PG's
    /// `uuid::Uuid` and stays portable for any sqlite twin.
    pub revision_id: String,
    /// Owning tenant, or [`BUNDLED_TENANT`] for system pages.
    pub tenant_id: String,
    /// Principal who can `edit` / `delete`; matches `created_by`
    /// for fresh writes, may diverge after `dashboard.duplicate`.
    pub owner_principal: String,
    /// Human page title (rendered in the route table; not parsed).
    pub title: String,
    /// Free-form tag list — filtered against in `list_active`.
    pub tags: Vec<String>,
    /// Wire body — `serde_json::Value` so the SPI is IR-agnostic.
    pub body_json: serde_json::Value,
    /// Principal who authored the revision.
    pub created_by: String,
    /// Server-side insertion time as RFC-3339; the column is
    /// `TIMESTAMPTZ` but the SPI keeps the value as a string to
    /// avoid pulling `chrono` / `time` into the contracts hub.
    pub created_at: String,
    /// `Some(rfc3339)` when this revision has been superseded by
    /// a newer row; `None` for the live head.
    pub superseded_at: Option<String>,
}

/// Payload for [`DashboardStore::insert_revision`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewRevision {
    /// SDUI page id; stable across revisions.
    pub page_id: String,
    /// Owning tenant; system rows use [`BUNDLED_TENANT`].
    pub tenant_id: String,
    /// Principal who can later `edit` / `delete` the page.
    pub owner_principal: String,
    /// Human title for the route table.
    pub title: String,
    /// Free-form tag list.
    pub tags: Vec<String>,
    /// `starter_ui_ir::ComponentTree` serialised to JSON.
    pub body_json: serde_json::Value,
    /// Principal that authored *this* revision (for audit). May
    /// differ from `owner_principal` when the AI builder writes
    /// under the flow caller's principal.
    pub created_by: String,
}

/// Filter for [`DashboardStore::list_active`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ListFilter {
    /// When non-empty, only return rows whose `tags` overlap any
    /// of these tags (GIN-indexed in the migration).
    pub tags_any: Vec<String>,
    /// When `Some`, only return rows whose `owner_principal`
    /// matches.
    pub owner: Option<String>,
}

/// Stable error surface for the `DashboardStore` trait. Variants
/// stay coarse on purpose — the structured fields are enough for
/// the tool body to decide whether to retry, surface 404, or
/// translate to `Conflict`.
#[derive(Debug, thiserror::Error)]
pub enum DashboardStoreError {
    /// The requested `(tenant_id, page_id)` has no live row.
    #[error("dashboard `{tenant_id}:{page_id}` not found")]
    NotFound {
        /// Tenant the lookup was scoped to.
        tenant_id: String,
        /// Page id that was missing.
        page_id: String,
    },
    /// Underlying storage failed (PG transport, encoding, etc.).
    /// `source` is opaque on purpose; callers should not pattern-
    /// match on it.
    #[error("dashboard store: {0}")]
    Backend(String),
}

/// Outcome of an [`DashboardStore::insert_revision_with_prior`]
/// call: the freshly-inserted revision plus the row it superseded,
/// if any. Returned by the chokepoint variant so the changelog
/// recorder can capture a byte-exact `before` snapshot without
/// re-fetching the row (which would re-introduce a TOCTOU window
/// between the supersede and the audit hand-off).
#[derive(Debug, Clone)]
pub struct InsertOutcome {
    /// The row written by the insert.
    pub inserted: DashboardRevision,
    /// The row that was live immediately before the insert and is
    /// now superseded — `None` if no prior row existed.
    pub prior: Option<DashboardRevision>,
}

/// Async trait every consumer (tool bodies, page resolver,
/// admin UI) dispatches through.
#[async_trait::async_trait]
pub trait DashboardStore: Send + Sync + 'static {
    /// Insert a fresh revision; if a live row already exists for
    /// `(tenant_id, page_id)` it is superseded in the same
    /// transaction. Returns the inserted row (so callers can echo
    /// `revision_id` to the user).
    async fn insert_revision(
        &self,
        new_revision: NewRevision,
    ) -> Result<DashboardRevision, DashboardStoreError>;

    /// Atomic variant of [`Self::insert_revision`] that also
    /// returns the row that was superseded (if any) so the
    /// changelog recorder can capture a byte-exact `before`
    /// snapshot for `Op::Update`. The default implementation calls
    /// [`Self::get_active`] then [`Self::insert_revision`] in
    /// sequence — backends that can do the read and the supersede
    /// in one transaction (Postgres `UPDATE ... RETURNING`) should
    /// override to eliminate the TOCTOU window. Tools that need
    /// audit fidelity (`rubix.dashboard.update`,
    /// `rubix.dashboard.patch`) call this; tools that don't (or
    /// callers that have already fetched the prior row themselves,
    /// like `rubix.dashboard.duplicate`) keep calling
    /// `insert_revision`.
    async fn insert_revision_with_prior(
        &self,
        new_revision: NewRevision,
    ) -> Result<InsertOutcome, DashboardStoreError> {
        let prior = self
            .get_active(&new_revision.tenant_id, &new_revision.page_id)
            .await?;
        let inserted = self.insert_revision(new_revision).await?;
        Ok(InsertOutcome { inserted, prior })
    }

    /// Return the single live revision for `(tenant_id, page_id)`,
    /// or `None` if no live row exists.
    async fn get_active(
        &self,
        tenant_id: &str,
        page_id: &str,
    ) -> Result<Option<DashboardRevision>, DashboardStoreError>;

    /// Return every live row for `tenant_id`, filtered by
    /// [`ListFilter`].
    async fn list_active(
        &self,
        tenant_id: &str,
        filter: &ListFilter,
    ) -> Result<Vec<DashboardRevision>, DashboardStoreError>;

    /// Return every live row across all tenants, filtered by
    /// [`ListFilter`]. The cross-tenant variant of
    /// [`Self::list_active`]; used by callers that already carry a
    /// super-admin (`tenant_id == "*"`) authorisation context
    /// (e.g. the dashboard-events SSE handler when the principal
    /// is a global Admin).
    ///
    /// The default implementation returns an empty list — backends
    /// that want to surface dashboards to super-admins (Postgres,
    /// the in-memory store) override it. Test fakes that never see
    /// a super-admin caller can rely on the default.
    async fn list_all_active(
        &self,
        _filter: &ListFilter,
    ) -> Result<Vec<DashboardRevision>, DashboardStoreError> {
        Ok(Vec::new())
    }

    /// Mark every live row for `(tenant_id, page_id)` as
    /// superseded *without* inserting a replacement (used by the
    /// `rubix.dashboard.delete` tool body). Returns the number of
    /// rows updated.
    async fn mark_superseded(
        &self,
        tenant_id: &str,
        page_id: &str,
    ) -> Result<u64, DashboardStoreError>;

    /// Return every revision (live and superseded) for `page_id`
    /// in `created_at DESC` order.
    async fn history(&self, page_id: &str) -> Result<Vec<DashboardRevision>, DashboardStoreError>;
}
