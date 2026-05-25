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
            other => Err(Error::Invalid {
                message: format!(
                    "DashboardReversible: unsupported op {other:?} (expected Create or Update)"
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
            other => Err(Error::Invalid {
                message: format!(
                    "DashboardReversible: unsupported op {other:?} (expected Create or Update)"
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
