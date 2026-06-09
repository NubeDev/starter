//! [`Reversible`] for dashboards — the snapshot reference impl (WS-12).
//!
//! Dashboards are **pinned to snapshot** (ROADMAP §6a D2): `before`/`after` carry
//! the full [`DashboardRecord`] shape so WS-05 "restore to version N" has an
//! absolute state, not a patch chain. Undo of an update re-applies the `before`
//! snapshot; redo re-applies `after`. Create/delete flip existence.
//!
//! The tenant is read from `ch.resource.tenant` (the changelog codec surfaces the
//! row's `tenant_id` there) so every store call binds the correct tenant under
//! RLS — the [`Reversible`] trait hands us only a [`Change`].

use async_trait::async_trait;
use serde_json::json;
use sqlx::PgPool;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Reversible};
use starter_spi::{Error, Result};
use uuid::Uuid;

use nexus_store::dashboard::{self, DashboardPatch, DashboardRecord, NewDashboard};

use crate::authz::KIND_DASHBOARD;

/// Snapshot-strategy [`Reversible`] for dashboards. Closes over the metadata pool
/// so inverses apply against the store inside a tenant transaction.
pub struct DashboardReversible {
    metadata: PgPool,
}

impl DashboardReversible {
    /// Build the reversible over the metadata pool (one instance, registered once
    /// at boot).
    pub fn new(metadata: PgPool) -> Self {
        Self { metadata }
    }

    /// Re-apply a full dashboard snapshot onto the live row. Used by both undo
    /// (snapshot = `before`) and redo (snapshot = `after`): a snapshot kind has
    /// no direction-specific logic beyond which side it writes. `None` when the
    /// snapshot is absent (a create has no `before`; a delete has no `after`),
    /// which the op-specific paths handle separately.
    async fn restore_snapshot(&self, tenant: &str, snapshot: &DashboardRecord) -> Result<()> {
        let patch = DashboardPatch {
            name: Some(snapshot.name.clone()),
            slug: Some(snapshot.slug.clone()),
            icon: Some(snapshot.icon.clone()),
            accent: Some(snapshot.accent.clone()),
            folder_id: Some(snapshot.folder_id),
            starred: Some(snapshot.starred),
        };
        match dashboard::update(&self.metadata, tenant, snapshot.id, &patch).await? {
            Some(_) => Ok(()),
            None => Err(Error::NotFound {
                what: format!("dashboard {}", snapshot.id),
            }),
        }
    }

    /// Re-create a deleted dashboard under its **original** id from a snapshot, so
    /// panels and grants keyed on that id stay valid. Used by undo-of-delete and
    /// redo-of-create. The store's id-stable insert (WS-05) makes this faithful;
    /// the panels themselves are not resurrected here (a delete cascades them and
    /// they are not in the dashboard snapshot — a known limitation, see the
    /// module doc).
    async fn resurrect(&self, tenant: &str, snapshot: &DashboardRecord) -> Result<()> {
        let new = NewDashboard {
            slug: snapshot.slug.clone(),
            name: snapshot.name.clone(),
            icon: snapshot.icon.clone(),
            accent: snapshot.accent.clone(),
            folder_id: snapshot.folder_id,
        };
        dashboard::insert_with_id(&self.metadata, tenant, snapshot.id, &new)
            .await
            .map(drop)
    }
}

#[async_trait]
impl Reversible for DashboardReversible {
    fn kind(&self) -> &'static str {
        KIND_DASHBOARD
    }

    /// Undo. For an update, re-apply the `before` snapshot. For a create, delete
    /// the row the create produced. For a delete, the row is resurrected with its
    /// original id (panels and grants key on it) via
    /// [`resurrect`](Self::resurrect).
    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            starter_spi::changelog::Op::Update => {
                let before = snapshot_from(ch.before.as_ref(), "before")?;
                self.restore_snapshot(tenant, &before).await
            }
            starter_spi::changelog::Op::Create => {
                let id = id_of(ch)?;
                dashboard::delete(&self.metadata, tenant, id).await.map(drop)
            }
            starter_spi::changelog::Op::Delete => {
                let before = snapshot_from(ch.before.as_ref(), "before")?;
                self.resurrect(tenant, &before).await
            }
            starter_spi::changelog::Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Redo. The mirror of [`apply_inverse`]: re-apply `after` for an update,
    /// re-delete for a delete, re-create for a create (id-stable, via
    /// [`resurrect`](Self::resurrect)).
    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            starter_spi::changelog::Op::Update => {
                let after = snapshot_from(ch.after.as_ref(), "after")?;
                self.restore_snapshot(tenant, &after).await
            }
            starter_spi::changelog::Op::Delete => {
                let id = id_of(ch)?;
                dashboard::delete(&self.metadata, tenant, id).await.map(drop)
            }
            starter_spi::changelog::Op::Create => {
                let after = snapshot_from(ch.after.as_ref(), "after")?;
                self.resurrect(tenant, &after).await
            }
            starter_spi::changelog::Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Duplicate. The `Reversible::clone_with` group path is not used for
    /// dashboards — duplication runs through the dedicated
    /// `POST /dashboards/:slug/duplicate` route (WS-05), which copies the
    /// dashboard *and its panels* under a fresh id and records its own `Change`.
    /// A bare clone here would copy only the dashboard row, orphaning the panels,
    /// so it is intentionally declined.
    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "dashboards duplicate via POST /dashboards/:slug/duplicate, \
                      not the changelog clone path (panels would be orphaned)"
                .into(),
        })
    }
}

/// Read the change's tenant, surfaced onto [`ResourceRef::tenant`] by the
/// changelog codec. Absent means the row was recorded without a tenant binding —
/// a substrate bug, not a caller error.
fn tenant_of(ch: &Change) -> Result<&str> {
    ch.resource.tenant.as_deref().ok_or_else(|| Error::Internal {
        source: "dashboard change is missing its tenant binding".into(),
    })
}

/// Parse the change's resource id as a dashboard [`Uuid`].
fn id_of(ch: &Change) -> Result<Uuid> {
    let raw = ch.resource.id.as_deref().ok_or_else(|| Error::Internal {
        source: "dashboard change is missing its resource id".into(),
    })?;
    Uuid::parse_str(raw).map_err(|e| Error::Internal {
        source: format!("dashboard change has a non-uuid resource id {raw:?}: {e}").into(),
    })
}

/// Decode a `before`/`after` JSON snapshot into a [`DashboardRecord`]. The
/// `which` label names the side for a precise error if the snapshot is missing or
/// malformed (e.g. an update recorded without a pre-read).
fn snapshot_from(value: Option<&serde_json::Value>, which: &str) -> Result<DashboardRecord> {
    let v = value.ok_or_else(|| Error::Invalid {
        message: format!("dashboard change has no {which} snapshot to apply"),
    })?;
    let id = field_str(v, "id")?;
    // folder_id is optional (NULL = root) and starred defaults false, so both are
    // read leniently: an older snapshot recorded before these fields existed
    // restores to root/unstarred rather than failing.
    let folder_id = match v.get("folder_id").and_then(|f| f.as_str()) {
        Some(raw) => Some(Uuid::parse_str(raw).map_err(|e| Error::Invalid {
            message: format!("dashboard {which} snapshot folder_id {raw:?} is not a uuid: {e}"),
        })?),
        None => None,
    };
    Ok(DashboardRecord {
        id: Uuid::parse_str(&id).map_err(|e| Error::Invalid {
            message: format!("dashboard {which} snapshot id {id:?} is not a uuid: {e}"),
        })?,
        tenant_id: field_str(v, "tenant_id")?,
        slug: field_str(v, "slug")?,
        name: field_str(v, "name")?,
        icon: field_str(v, "icon")?,
        accent: field_str(v, "accent")?,
        folder_id,
        starred: v.get("starred").and_then(|s| s.as_bool()).unwrap_or(false),
    })
}

/// Pull a required string field from a snapshot object, naming the field on a
/// type/absence mismatch so a malformed recording is diagnosable.
fn field_str(v: &serde_json::Value, field: &str) -> Result<String> {
    v.get(field)
        .and_then(|f| f.as_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid {
            message: format!("dashboard snapshot is missing string field {field:?}"),
        })
}

/// The JSON shape a recording handler must capture so this impl can reverse it.
/// Kept here next to the decoder so the producing side and consuming side stay in
/// step. Exposed for the recording handlers and the coverage guard.
pub fn snapshot_json(rec: &DashboardRecord) -> serde_json::Value {
    json!({
        "id": rec.id.to_string(),
        "tenant_id": rec.tenant_id,
        "slug": rec.slug,
        "name": rec.name,
        "icon": rec.icon,
        "accent": rec.accent,
        "folder_id": rec.folder_id.map(|id| id.to_string()),
        "starred": rec.starred,
    })
}

/// Custom ops have no defined inverse for this kind.
fn custom_unsupported(custom: &str) -> Error {
    Error::Invalid {
        message: format!("dashboard has no reversible for custom op {custom:?}"),
    }
}
