//! [`Reversible`] for panels — the snapshot impl that makes dashboard editing
//! actually undoable (WS-12 follow-up).
//!
//! Panels are **snapshot** (small, lifecycle-shaped: create/delete flip
//! existence; an update swaps title/sql/viz/datasource/layout). The recorded
//! `before`/`after` carry the full [`PanelRecord`]. Panels hold no secrets, so —
//! unlike datasources — delete is fully reversible: undo-of-delete resurrects the
//! panel under its **original** id (the dashboard's layout JSON addresses panels
//! by id) via [`panel::insert_with_id`](nexus_store::dashboard::panel::insert_with_id).
//!
//! Why a dedicated panel kind at all: before this impl existed, panel edits were
//! not recorded, so an Undo issued while editing a dashboard fell through to the
//! dashboard's own `Create` row and **deleted the whole dashboard**. Recording
//! panels under [`KIND_PANEL`] makes undo revert the last panel edit instead.
//!
//! The tenant is read from `ch.resource.tenant` (surfaced by the changelog codec)
//! so store calls bind the caller's tenant under RLS.

use async_trait::async_trait;
use serde_json::json;
use sqlx::PgPool;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::{Error, Result};
use uuid::Uuid;

use nexus_store::dashboard::{panel, NewPanel, PanelPatch, PanelRecord};

use crate::authz::KIND_PANEL;

/// Snapshot-strategy [`Reversible`] for panels. Closes over the metadata pool so
/// inverses apply against the store inside a tenant transaction.
pub struct PanelReversible {
    metadata: PgPool,
}

impl PanelReversible {
    /// Build the reversible over the metadata pool (one instance, registered once
    /// at boot).
    pub fn new(metadata: PgPool) -> Self {
        Self { metadata }
    }

    /// Re-apply a full panel snapshot onto the live row. Used by both undo
    /// (snapshot = `before`) and redo (snapshot = `after`): a snapshot kind has no
    /// direction-specific logic beyond which side it writes. `NotFound` if the row
    /// is gone. `layout`/`sql`/`viz` are always set (full snapshot), so the
    /// COALESCE-on-`None` store semantics never leave a stale field behind. The
    /// insight fields use the three-valued patch with `Some(_)` so the snapshot's
    /// value is restored *exactly* — including a `None` (an insight that was
    /// detached at the snapshot point), which a plain COALESCE could not reinstate.
    async fn restore_snapshot(&self, tenant: &str, snapshot: &PanelRecord) -> Result<()> {
        let patch = PanelPatch {
            title: Some(snapshot.title.clone()),
            datasource_id: snapshot.datasource_id,
            sql: Some(snapshot.sql.clone()),
            viz: Some(snapshot.viz.clone()),
            layout: Some(snapshot.layout.clone()),
            insight_id: Some(snapshot.insight_id),
            insight_params: Some(snapshot.insight_params.clone()),
        };
        match panel::update(&self.metadata, tenant, snapshot.id, &patch).await? {
            Some(_) => Ok(()),
            None => Err(Error::NotFound {
                what: format!("panel {}", snapshot.id),
            }),
        }
    }

    /// Re-create a deleted panel under its **original** id from a snapshot, so the
    /// dashboard's layout JSON (which keys panels by id) still addresses it. Used
    /// by undo-of-delete and redo-of-create.
    async fn resurrect(&self, tenant: &str, snapshot: &PanelRecord) -> Result<()> {
        let new = NewPanel {
            dashboard_id: snapshot.dashboard_id,
            datasource_id: snapshot.datasource_id,
            title: snapshot.title.clone(),
            sql: snapshot.sql.clone(),
            viz: snapshot.viz.clone(),
            layout: snapshot.layout.clone(),
            insight_id: snapshot.insight_id,
            insight_params: snapshot.insight_params.clone(),
        };
        panel::insert_with_id(&self.metadata, tenant, snapshot.id, &new)
            .await
            .map(drop)
    }
}

#[async_trait]
impl Reversible for PanelReversible {
    fn kind(&self) -> &'static str {
        KIND_PANEL
    }

    /// Undo. For an update, re-apply the `before` snapshot. For a create, delete
    /// the row the create produced. For a delete, resurrect the row under its
    /// original id from the `before` snapshot.
    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            Op::Update => {
                let before = snapshot_from(ch.before.as_ref(), "before")?;
                self.restore_snapshot(tenant, &before).await
            }
            Op::Create => {
                let id = id_of(ch)?;
                panel::delete(&self.metadata, tenant, id).await.map(drop)
            }
            Op::Delete => {
                let before = snapshot_from(ch.before.as_ref(), "before")?;
                self.resurrect(tenant, &before).await
            }
            Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Redo. The mirror of [`apply_inverse`]: re-apply `after` for an update,
    /// re-delete for a delete, re-create (id-stable) for a create.
    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            Op::Update => {
                let after = snapshot_from(ch.after.as_ref(), "after")?;
                self.restore_snapshot(tenant, &after).await
            }
            Op::Delete => {
                let id = id_of(ch)?;
                panel::delete(&self.metadata, tenant, id).await.map(drop)
            }
            Op::Create => {
                let after = snapshot_from(ch.after.as_ref(), "after")?;
                self.resurrect(tenant, &after).await
            }
            Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Duplicate via the changelog clone path is not used for panels — the canvas
    /// adds panels through `POST /dashboards/:slug/panels`, which records its own
    /// `Change`. A bare clone here would need a target dashboard the clone API
    /// can't express, so it is declined.
    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "panels are added via POST /dashboards/:slug/panels, \
                      not the changelog clone path"
                .into(),
        })
    }
}

/// Read the change's tenant binding (surfaced by the codec). Absent = substrate
/// bug.
fn tenant_of(ch: &Change) -> Result<&str> {
    ch.resource.tenant.as_deref().ok_or_else(|| Error::Internal {
        source: "panel change is missing its tenant binding".into(),
    })
}

/// Parse the change's resource id as a panel [`Uuid`].
fn id_of(ch: &Change) -> Result<Uuid> {
    let raw = ch.resource.id.as_deref().ok_or_else(|| Error::Internal {
        source: "panel change is missing its resource id".into(),
    })?;
    Uuid::parse_str(raw).map_err(|e| Error::Internal {
        source: format!("panel change has a non-uuid resource id {raw:?}: {e}").into(),
    })
}

/// Decode a `before`/`after` JSON snapshot into a [`PanelRecord`]. The `which`
/// label names the side for a precise error if the snapshot is missing or
/// malformed (e.g. an update recorded without a pre-read).
fn snapshot_from(value: Option<&serde_json::Value>, which: &str) -> Result<PanelRecord> {
    let v = value.ok_or_else(|| Error::Invalid {
        message: format!("panel change has no {which} snapshot to apply"),
    })?;
    let id = field_uuid(v, "id", which)?;
    let dashboard_id = field_uuid(v, "dashboard_id", which)?;
    // datasource_id is optional (a panel can exist without a bound datasource).
    let datasource_id = match v.get("datasource_id").and_then(|d| d.as_str()) {
        Some(raw) => Some(Uuid::parse_str(raw).map_err(|e| Error::Invalid {
            message: format!("panel {which} snapshot datasource_id {raw:?} is not a uuid: {e}"),
        })?),
        None => None,
    };
    // layout is opaque grid JSON the canvas owns — carried through verbatim.
    let layout = v.get("layout").cloned().unwrap_or_else(|| json!({}));
    // insight_id is optional (a panel may have no insight attached).
    let insight_id = match v.get("insight_id").and_then(|d| d.as_str()) {
        Some(raw) => Some(Uuid::parse_str(raw).map_err(|e| Error::Invalid {
            message: format!("panel {which} snapshot insight_id {raw:?} is not a uuid: {e}"),
        })?),
        None => None,
    };
    // insight_params is opaque JSON the script binds; carried through verbatim.
    // Absent or JSON null both mean "no params".
    let insight_params = match v.get("insight_params") {
        Some(serde_json::Value::Null) | None => None,
        Some(other) => Some(other.clone()),
    };
    Ok(PanelRecord {
        id,
        dashboard_id,
        datasource_id,
        title: field_str(v, "title")?,
        sql: field_str(v, "sql")?,
        viz: field_str(v, "viz")?,
        layout,
        insight_id,
        insight_params,
    })
}

/// Pull a required string field from a snapshot object, naming it on a
/// type/absence mismatch so a malformed recording is diagnosable.
fn field_str(v: &serde_json::Value, field: &str) -> Result<String> {
    v.get(field)
        .and_then(|f| f.as_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid {
            message: format!("panel snapshot is missing string field {field:?}"),
        })
}

/// Pull a required uuid field from a snapshot object.
fn field_uuid(v: &serde_json::Value, field: &str, which: &str) -> Result<Uuid> {
    let raw = field_str(v, field)?;
    Uuid::parse_str(&raw).map_err(|e| Error::Invalid {
        message: format!("panel {which} snapshot {field} {raw:?} is not a uuid: {e}"),
    })
}

/// The JSON shape a recording handler must capture so this impl can reverse it.
/// Kept next to the decoder so the producing and consuming sides stay in step.
/// Exposed for the recording handlers and the coverage guard. Panels carry no
/// secrets, so the whole record is recorded verbatim.
pub fn snapshot_json(rec: &PanelRecord) -> serde_json::Value {
    json!({
        "id": rec.id.to_string(),
        "dashboard_id": rec.dashboard_id.to_string(),
        "datasource_id": rec.datasource_id.map(|id| id.to_string()),
        "title": rec.title,
        "sql": rec.sql,
        "viz": rec.viz,
        "layout": rec.layout,
        "insight_id": rec.insight_id.map(|id| id.to_string()),
        "insight_params": rec.insight_params,
    })
}

/// Custom ops have no defined inverse for this kind.
fn custom_unsupported(custom: &str) -> Error {
    Error::Invalid {
        message: format!("panel has no reversible for custom op {custom:?}"),
    }
}
