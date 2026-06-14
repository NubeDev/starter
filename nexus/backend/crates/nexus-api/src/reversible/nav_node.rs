//! [`Reversible`] for nav nodes — a snapshot-strategy impl (WS-13 / C6).
//!
//! Nav nodes are small, lifecycle-shaped rows (create/delete flip existence,
//! update retitles/reparents/retargets), so like folders they are reversed from
//! a full **snapshot** rather than a patch chain: `before`/`after` carry the
//! whole [`NavNodeRecord`] shape, including the opaque `target`/`context` JSONB.
//! Undo of an update re-applies the `before` snapshot; redo re-applies `after`.
//! Create/delete flip existence (delete resurrects under the original id so
//! re-rooted children can be re-parented by a subsequent undo).
//!
//! The tenant is read from `ch.resource.tenant` so every store call binds the
//! correct tenant under RLS.

use async_trait::async_trait;
use serde_json::json;
use sqlx::PgPool;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::{Error, Result};
use uuid::Uuid;

use nexus_store::nav_node::{self, NavNodePatch, NavNodeRecord, NewNavNode};

use crate::authz::KIND_NAV_NODE;

/// Snapshot-strategy [`Reversible`] for nav nodes. Closes over the metadata pool
/// so inverses apply against the store inside a tenant transaction.
pub struct NavNodeReversible {
    metadata: PgPool,
}

impl NavNodeReversible {
    /// Build the reversible over the metadata pool (one instance, registered once
    /// at boot).
    pub fn new(metadata: PgPool) -> Self {
        Self { metadata }
    }

    /// Re-apply a full snapshot onto the live row (undo = `before`, redo =
    /// `after`). Parent/context/icon/accent are set explicitly via the store's
    /// three-valued patch so a snapshot can re-root or clear them — COALESCE
    /// can't express either.
    async fn restore_snapshot(&self, tenant: &str, snap: &NavNodeRecord) -> Result<()> {
        let patch = NavNodePatch {
            parent_id: Some(snap.parent_id),
            title: Some(snap.title.clone()),
            sort_order: Some(snap.sort_order),
            target: Some(snap.target.clone()),
            context: Some(snap.context.clone()),
            icon: Some(snap.icon.clone()),
            accent: Some(snap.accent.clone()),
        };
        match nav_node::update(&self.metadata, tenant, snap.id, &patch).await? {
            Some(_) => Ok(()),
            None => Err(Error::NotFound {
                what: format!("nav node {}", snap.id),
            }),
        }
    }

    /// Re-create a deleted node under its **original** id so a subsequent undo can
    /// re-parent the children the delete re-rooted, and a redo-of-create restores
    /// the same id.
    async fn resurrect(&self, tenant: &str, snap: &NavNodeRecord) -> Result<()> {
        let new = NewNavNode {
            parent_id: snap.parent_id,
            title: snap.title.clone(),
            sort_order: snap.sort_order,
            target: snap.target.clone(),
            context: snap.context.clone(),
            icon: snap.icon.clone(),
            accent: snap.accent.clone(),
        };
        nav_node::insert_with_id(&self.metadata, tenant, snap.id, &new)
            .await
            .map(drop)
    }
}

#[async_trait]
impl Reversible for NavNodeReversible {
    fn kind(&self) -> &'static str {
        KIND_NAV_NODE
    }

    /// Undo. Update → re-apply `before`; create → delete the produced row;
    /// delete → resurrect with the original id.
    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            Op::Update => {
                let before = snapshot_from(ch.before.as_ref(), "before")?;
                self.restore_snapshot(tenant, &before).await
            }
            Op::Create => {
                let id = id_of(ch)?;
                nav_node::delete(&self.metadata, tenant, id).await.map(drop)
            }
            Op::Delete => {
                let before = snapshot_from(ch.before.as_ref(), "before")?;
                self.resurrect(tenant, &before).await
            }
            Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Redo. The mirror of [`apply_inverse`].
    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            Op::Update => {
                let after = snapshot_from(ch.after.as_ref(), "after")?;
                self.restore_snapshot(tenant, &after).await
            }
            Op::Delete => {
                let id = id_of(ch)?;
                nav_node::delete(&self.metadata, tenant, id).await.map(drop)
            }
            Op::Create => {
                let after = snapshot_from(ch.after.as_ref(), "after")?;
                self.resurrect(tenant, &after).await
            }
            Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Nav nodes have no duplicate flow, so the changelog clone path is declined.
    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "nav nodes are not duplicable".into(),
        })
    }
}

/// Read the change's tenant, surfaced onto [`ResourceRef::tenant`] by the codec.
fn tenant_of(ch: &Change) -> Result<&str> {
    ch.resource
        .tenant
        .as_deref()
        .ok_or_else(|| Error::Internal {
            source: "nav node change is missing its tenant binding".into(),
        })
}

/// Parse the change's resource id as a nav-node [`Uuid`].
fn id_of(ch: &Change) -> Result<Uuid> {
    let raw = ch.resource.id.as_deref().ok_or_else(|| Error::Internal {
        source: "nav node change is missing its resource id".into(),
    })?;
    Uuid::parse_str(raw).map_err(|e| Error::Internal {
        source: format!("nav node change has a non-uuid resource id {raw:?}: {e}").into(),
    })
}

/// Decode a `before`/`after` JSON snapshot into a [`NavNodeRecord`]. The `which`
/// label names the side for a precise error if the snapshot is missing/malformed.
fn snapshot_from(value: Option<&serde_json::Value>, which: &str) -> Result<NavNodeRecord> {
    let v = value.ok_or_else(|| Error::Invalid {
        message: format!("nav node change has no {which} snapshot to apply"),
    })?;
    let parent_id = parse_opt_uuid(v.get("parent_id"), which, "parent_id")?;
    Ok(NavNodeRecord {
        id: parse_uuid(field_str(v, "id")?, which, "id")?,
        tenant_id: field_str(v, "tenant_id")?,
        parent_id,
        title: field_str(v, "title")?,
        sort_order: v.get("sort_order").and_then(|s| s.as_i64()).unwrap_or(0) as i32,
        target: v
            .get("target")
            .cloned()
            .unwrap_or(json!({ "kind": "group" })),
        context: v.get("context").cloned().filter(|c| !c.is_null()),
        icon: v.get("icon").and_then(|s| s.as_str()).map(str::to_owned),
        accent: v.get("accent").and_then(|s| s.as_str()).map(str::to_owned),
    })
}

fn parse_opt_uuid(
    field: Option<&serde_json::Value>,
    which: &str,
    name: &str,
) -> Result<Option<Uuid>> {
    match field.and_then(|p| p.as_str()) {
        Some(raw) => Ok(Some(parse_uuid(raw.to_owned(), which, name)?)),
        None => Ok(None),
    }
}

fn parse_uuid(raw: String, which: &str, name: &str) -> Result<Uuid> {
    Uuid::parse_str(&raw).map_err(|e| Error::Invalid {
        message: format!("nav node {which} snapshot {name} {raw:?} is not a uuid: {e}"),
    })
}

/// Pull a required string field, naming it on a type/absence mismatch.
fn field_str(v: &serde_json::Value, field: &str) -> Result<String> {
    v.get(field)
        .and_then(|f| f.as_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid {
            message: format!("nav node snapshot is missing string field {field:?}"),
        })
}

/// The JSON shape a recording handler must capture so this impl can reverse it.
/// Kept next to the decoder so producer and consumer stay in step.
pub fn snapshot_json(rec: &NavNodeRecord) -> serde_json::Value {
    json!({
        "id": rec.id.to_string(),
        "tenant_id": rec.tenant_id,
        "parent_id": rec.parent_id.map(|id| id.to_string()),
        "title": rec.title,
        "sort_order": rec.sort_order,
        "target": rec.target,
        "context": rec.context,
        "icon": rec.icon,
        "accent": rec.accent,
    })
}

/// Custom ops have no defined inverse for this kind.
fn custom_unsupported(custom: &str) -> Error {
    Error::Invalid {
        message: format!("nav node has no reversible for custom op {custom:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(parent: Option<Uuid>) -> NavNodeRecord {
        NavNodeRecord {
            id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            tenant_id: "acme".into(),
            parent_id: parent,
            title: "Building-1".into(),
            sort_order: 3,
            target: json!({ "kind": "dashboard", "dashboardId": "deadbeef-0000-0000-0000-000000000000" }),
            context: Some(json!({ "values": { "building": "b1" } })),
            icon: Some("Activity".into()),
            accent: None,
        }
    }

    /// What the recording handler emits must decode back to the same record, so
    /// producer (`snapshot_json`) and consumer (`snapshot_from`) stay in lockstep.
    /// Covers a nested dashboard mount and a root group node.
    #[test]
    fn snapshot_round_trips_for_mount_and_root_group() {
        let nested = rec(Some(
            Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        ));
        let mut root_group = rec(None);
        root_group.target = json!({ "kind": "group" });
        root_group.context = None;

        for original in [nested, root_group] {
            let json = snapshot_json(&original);
            let decoded = snapshot_from(Some(&json), "before").expect("decode");
            assert_eq!(decoded.id, original.id);
            assert_eq!(decoded.tenant_id, original.tenant_id);
            assert_eq!(decoded.parent_id, original.parent_id);
            assert_eq!(decoded.title, original.title);
            assert_eq!(decoded.sort_order, original.sort_order);
            assert_eq!(decoded.target, original.target);
            assert_eq!(decoded.context, original.context);
            assert_eq!(decoded.icon, original.icon);
            assert_eq!(decoded.accent, original.accent);
        }
    }

    /// A missing snapshot is a precise, named error rather than a panic.
    #[test]
    fn a_missing_snapshot_is_an_invalid_error() {
        let err = snapshot_from(None, "before").unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    /// The impl reports the nav-node kind the registry and coverage guard expect.
    #[test]
    fn kind_is_the_nav_node_discriminator() {
        assert_eq!(KIND_NAV_NODE, "nexus.nav_node");
    }
}
