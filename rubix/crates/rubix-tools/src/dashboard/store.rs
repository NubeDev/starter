//! [`Reversible`] glue for `rubix.dashboard.*` write verbs.
//!
//! The two write verbs landed in Phase C.2 (`dashboard.create` and
//! `dashboard.update`) record a [`ChangeDraft`] through
//! [`ReversibleTool::change_for`]. The undo dispatcher then looks up
//! a [`Reversible`] impl keyed on the resource kind
//! (`rubix.dashboard.page`) to actually walk the change backwards or
//! forwards.
//!
//! [`DashboardReversible`] dispatches both shapes:
//!
//! - **`Op::Create`** — undo by superseding every live row for
//!   `(tenant_id, page_id)` (i.e. soft-delete via
//!   [`DashboardStore::mark_superseded`]); redo by re-inserting the
//!   `after` snapshot via [`DashboardStore::insert_revision`].
//! - **`Op::Update`** — undo by inserting a fresh revision whose
//!   body is the `before` snapshot (the insert-only store
//!   automatically supersedes the post-update head); redo by
//!   inserting the `after` snapshot back on top.
//!
//! Both shapes use the `(tenant_id, page_id)` pair carried in
//! [`DashboardSnapshot`] for the lookup so the SDUI `page_id` stays
//! stable across revisions per the storage scope.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::{DashboardStore, NewRevision};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::error::{Error, Result};

/// Resource-kind discriminator for SDUI dashboard pages. Mirrors
/// the `rubix.dashboard.page` `ResourceSpec` registered eagerly at
/// boot in `rubix-agent::boot::authz`.
pub const DASHBOARD_PAGE_KIND: &str = "rubix.dashboard.page";

/// Snapshot payload stamped into [`Change::before`] / [`Change::after`].
///
/// Carries every field needed to rebuild a [`NewRevision`] so the
/// `Reversible` impl can re-insert without re-querying. `revision_id`
/// is informational (debug/audit only — the store mints a new
/// revision id on every insert).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    /// Stable SDUI page id.
    pub page_id: String,
    /// Tenant the row belongs to.
    pub tenant_id: String,
    /// Principal who can `edit` / `delete`.
    pub owner_principal: String,
    /// Human title at this revision.
    pub title: String,
    /// Tag list at this revision.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Body bytes at this revision.
    pub body_json: serde_json::Value,
    /// Principal who authored this revision.
    pub created_by: String,
    /// Revision id of the row this snapshot was taken from. Echoed
    /// back to the audit log; not consulted by the inverse path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<String>,
}

impl DashboardSnapshot {
    /// Convert into the `NewRevision` payload the store expects.
    pub fn into_new_revision(self) -> NewRevision {
        NewRevision {
            page_id: self.page_id,
            tenant_id: self.tenant_id,
            owner_principal: self.owner_principal,
            title: self.title,
            tags: self.tags,
            body_json: self.body_json,
            created_by: self.created_by,
        }
    }
}

/// Single [`Reversible`] impl for the `rubix.dashboard.page` kind.
pub struct DashboardReversible {
    store: Arc<dyn DashboardStore>,
}

impl DashboardReversible {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn DashboardStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Reversible for DashboardReversible {
    fn kind(&self) -> &'static str {
        DASHBOARD_PAGE_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        match &ch.op {
            Op::Create => {
                let snap: DashboardSnapshot = parse(ch.after.as_ref(), "after")?;
                self.store
                    .mark_superseded(&snap.tenant_id, &snap.page_id)
                    .await
                    .map_err(backend)?;
                Ok(())
            }
            Op::Update => {
                let snap: DashboardSnapshot = parse(ch.before.as_ref(), "before")?;
                self.store
                    .insert_revision(snap.into_new_revision())
                    .await
                    .map_err(backend)?;
                Ok(())
            }
            Op::Delete => {
                // Undo of a delete: re-insert the prior live row
                // (carried in `before`). The store mints a fresh
                // `revision_id`; the SDUI `page_id` is stable so
                // any cached `page_ref` still resolves.
                let snap: DashboardSnapshot = parse(ch.before.as_ref(), "before")?;
                self.store
                    .insert_revision(snap.into_new_revision())
                    .await
                    .map_err(backend)?;
                Ok(())
            }
            other => Err(Error::Invalid {
                message: format!(
                    "DashboardReversible: unsupported op {other:?} (expected Create, Update or Delete)"
                ),
            }),
        }
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        match &ch.op {
            Op::Create | Op::Update => {
                let snap: DashboardSnapshot = parse(ch.after.as_ref(), "after")?;
                self.store
                    .insert_revision(snap.into_new_revision())
                    .await
                    .map_err(backend)?;
                Ok(())
            }
            Op::Delete => {
                // Redo of a delete: re-supersede the page using the
                // tenant/page coordinates carried in `before`.
                let snap: DashboardSnapshot = parse(ch.before.as_ref(), "before")?;
                self.store
                    .mark_superseded(&snap.tenant_id, &snap.page_id)
                    .await
                    .map_err(backend)?;
                Ok(())
            }
            other => Err(Error::Invalid {
                message: format!(
                    "DashboardReversible: unsupported op {other:?} (expected Create, Update or Delete)"
                ),
            }),
        }
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        // The `rubix.dashboard.duplicate` verb owns the clone path
        // (Phase C.3); the changelog-level clone is intentionally
        // unwired.
        Err(Error::Invalid {
            message: "rubix.dashboard.page kind: use rubix.dashboard.duplicate for clones".into(),
        })
    }
}

fn parse<T: for<'de> Deserialize<'de>>(payload: Option<&Value>, field: &str) -> Result<T> {
    let v = payload.ok_or_else(|| Error::Invalid {
        message: format!("DashboardReversible: Change::{field} is None"),
    })?;
    serde_json::from_value::<T>(v.clone()).map_err(|e| Error::Invalid {
        message: format!("DashboardReversible: Change::{field} parse: {e}"),
    })
}

fn backend(e: rubix_spi::dashboard::DashboardStoreError) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

/// In-memory [`DashboardStore`] suitable for laptop / no-Postgres
/// boots and tests that do not need durability. Mirrors the PG
/// insert-only contract: `insert_revision` supersedes any prior
/// live row for `(tenant_id, page_id)` in the same critical
/// section; `get_active` / `list_active` skip superseded rows;
/// `history` returns every revision newest-first.
///
/// Used by [`rubix_agent::registry::build_tool_registry`] as the
/// fallback backing store when no PG pool is wired. The PG-backed
/// [`rubix_store_postgres::PgDashboardStore`] is the production
/// drop-in replacement (same trait shape).
#[derive(Default)]
pub struct InMemoryDashboardStore {
    rows: std::sync::Mutex<Vec<rubix_spi::dashboard::DashboardRevision>>,
    next_rev: std::sync::Mutex<u64>,
}

impl InMemoryDashboardStore {
    /// Fresh empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn next_rev(&self) -> String {
        let mut g = self.next_rev.lock().unwrap();
        *g += 1;
        format!("mem-rev-{g}")
    }

    /// Monotonic timestamp string. Wall-clock fidelity does not
    /// matter — the contract is that newer inserts compare greater
    /// than older ones via lexical `cmp` so `history` ordering
    /// works.
    fn now() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        // Pad so lex order matches numeric order across at least the
        // next century.
        format!("{nanos:030}")
    }
}

#[async_trait]
impl DashboardStore for InMemoryDashboardStore {
    async fn insert_revision(
        &self,
        new: NewRevision,
    ) -> std::result::Result<
        rubix_spi::dashboard::DashboardRevision,
        rubix_spi::dashboard::DashboardStoreError,
    > {
        let now = Self::now();
        let mut rows = self.rows.lock().unwrap();
        for r in rows.iter_mut() {
            if r.tenant_id == new.tenant_id && r.page_id == new.page_id && r.superseded_at.is_none()
            {
                r.superseded_at = Some(now.clone());
            }
        }
        let inserted = rubix_spi::dashboard::DashboardRevision {
            page_id: new.page_id,
            revision_id: self.next_rev(),
            tenant_id: new.tenant_id,
            owner_principal: new.owner_principal,
            title: new.title,
            tags: new.tags,
            body_json: new.body_json,
            created_by: new.created_by,
            created_at: now,
            superseded_at: None,
        };
        rows.push(inserted.clone());
        Ok(inserted)
    }

    /// Atomic variant — captures the prior row in the same lock
    /// scope as the supersede + insert, so the audit recorder
    /// sees a coherent before-state without any race window.
    async fn insert_revision_with_prior(
        &self,
        new: NewRevision,
    ) -> std::result::Result<
        rubix_spi::dashboard::InsertOutcome,
        rubix_spi::dashboard::DashboardStoreError,
    > {
        let now = Self::now();
        let mut rows = self.rows.lock().unwrap();
        let mut prior: Option<rubix_spi::dashboard::DashboardRevision> = None;
        for r in rows.iter_mut() {
            if r.tenant_id == new.tenant_id && r.page_id == new.page_id && r.superseded_at.is_none()
            {
                prior = Some(r.clone());
                r.superseded_at = Some(now.clone());
            }
        }
        let inserted = rubix_spi::dashboard::DashboardRevision {
            page_id: new.page_id,
            revision_id: self.next_rev(),
            tenant_id: new.tenant_id,
            owner_principal: new.owner_principal,
            title: new.title,
            tags: new.tags,
            body_json: new.body_json,
            created_by: new.created_by,
            created_at: now,
            superseded_at: None,
        };
        rows.push(inserted.clone());
        Ok(rubix_spi::dashboard::InsertOutcome { inserted, prior })
    }

    async fn get_active(
        &self,
        tenant_id: &str,
        page_id: &str,
    ) -> std::result::Result<
        Option<rubix_spi::dashboard::DashboardRevision>,
        rubix_spi::dashboard::DashboardStoreError,
    > {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.tenant_id == tenant_id && r.page_id == page_id && r.superseded_at.is_none())
            .cloned())
    }

    async fn list_active(
        &self,
        tenant_id: &str,
        filter: &rubix_spi::dashboard::ListFilter,
    ) -> std::result::Result<
        Vec<rubix_spi::dashboard::DashboardRevision>,
        rubix_spi::dashboard::DashboardStoreError,
    > {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.tenant_id == tenant_id && r.superseded_at.is_none())
            .filter(|r| {
                filter.tags_any.is_empty() || r.tags.iter().any(|t| filter.tags_any.contains(t))
            })
            .filter(|r| match &filter.owner {
                None => true,
                Some(o) => &r.owner_principal == o,
            })
            .cloned()
            .collect())
    }

    async fn list_all_active(
        &self,
        filter: &rubix_spi::dashboard::ListFilter,
    ) -> std::result::Result<
        Vec<rubix_spi::dashboard::DashboardRevision>,
        rubix_spi::dashboard::DashboardStoreError,
    > {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.superseded_at.is_none())
            .filter(|r| {
                filter.tags_any.is_empty() || r.tags.iter().any(|t| filter.tags_any.contains(t))
            })
            .filter(|r| match &filter.owner {
                None => true,
                Some(o) => &r.owner_principal == o,
            })
            .cloned()
            .collect())
    }

    async fn mark_superseded(
        &self,
        tenant_id: &str,
        page_id: &str,
    ) -> std::result::Result<u64, rubix_spi::dashboard::DashboardStoreError> {
        let now = Self::now();
        let mut n = 0u64;
        for r in self.rows.lock().unwrap().iter_mut() {
            if r.tenant_id == tenant_id && r.page_id == page_id && r.superseded_at.is_none() {
                r.superseded_at = Some(now.clone());
                n += 1;
            }
        }
        Ok(n)
    }

    async fn history(
        &self,
        page_id: &str,
    ) -> std::result::Result<
        Vec<rubix_spi::dashboard::DashboardRevision>,
        rubix_spi::dashboard::DashboardStoreError,
    > {
        let mut rows: Vec<_> = self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.page_id == page_id)
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(rows)
    }
}
