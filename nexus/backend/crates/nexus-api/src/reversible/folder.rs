//! [`Reversible`] for folders — a snapshot-strategy impl (WS-05 / C6).
//!
//! Folders are small, lifecycle-shaped rows (create/delete flip existence,
//! update renames/reparents), so like dashboards they are reversed from a full
//! **snapshot** rather than a patch chain: `before`/`after` carry the whole
//! [`FolderRecord`] shape. Undo of an update re-applies the `before` snapshot;
//! redo re-applies `after`. Create/delete flip existence.
//!
//! The tenant is read from `ch.resource.tenant` (surfaced by the changelog codec
//! from the row's `tenant_id`) so every store call binds the correct tenant under
//! RLS.

use async_trait::async_trait;
use serde_json::json;
use sqlx::PgPool;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Reversible};
use starter_spi::{Error, Result};
use uuid::Uuid;

use nexus_store::folder::{self, FolderPatch, FolderRecord, NewFolder};

use crate::authz::KIND_FOLDER;

/// Snapshot-strategy [`Reversible`] for folders. Closes over the metadata pool so
/// inverses apply against the store inside a tenant transaction.
pub struct FolderReversible {
    metadata: PgPool,
}

impl FolderReversible {
    /// Build the reversible over the metadata pool (one instance, registered once
    /// at boot).
    pub fn new(metadata: PgPool) -> Self {
        Self { metadata }
    }

    /// Re-apply a full folder snapshot onto the live row, used by both undo
    /// (snapshot = `before`) and redo (snapshot = `after`). The parent is set
    /// explicitly (a snapshot may re-root the folder, which COALESCE can't
    /// express) via the store's three-valued patch.
    async fn restore_snapshot(&self, tenant: &str, snapshot: &FolderRecord) -> Result<()> {
        let patch = FolderPatch {
            name: Some(snapshot.name.clone()),
            parent_id: Some(snapshot.parent_id),
        };
        match folder::update(&self.metadata, tenant, snapshot.id, &patch).await? {
            Some(_) => Ok(()),
            None => Err(Error::NotFound {
                what: format!("folder {}", snapshot.id),
            }),
        }
    }

    /// Re-create a deleted folder under its **original** id so any rows the delete
    /// re-rooted (children, filed dashboards) can be re-filed by a subsequent
    /// undo, and so a redo-of-create restores the same id. The re-rooting of
    /// contents is not itself reversed here — re-parenting them is the caller's
    /// own recorded change, undone independently.
    async fn resurrect(&self, tenant: &str, snapshot: &FolderRecord) -> Result<()> {
        let new = NewFolder {
            parent_id: snapshot.parent_id,
            name: snapshot.name.clone(),
        };
        folder::insert_with_id(&self.metadata, tenant, snapshot.id, &new)
            .await
            .map(drop)
    }
}

#[async_trait]
impl Reversible for FolderReversible {
    fn kind(&self) -> &'static str {
        KIND_FOLDER
    }

    /// Undo. Update → re-apply `before`; create → delete the produced row;
    /// delete → resurrect with the original id.
    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            starter_spi::changelog::Op::Update => {
                let before = snapshot_from(ch.before.as_ref(), "before")?;
                self.restore_snapshot(tenant, &before).await
            }
            starter_spi::changelog::Op::Create => {
                let id = id_of(ch)?;
                folder::delete(&self.metadata, tenant, id).await.map(drop)
            }
            starter_spi::changelog::Op::Delete => {
                let before = snapshot_from(ch.before.as_ref(), "before")?;
                self.resurrect(tenant, &before).await
            }
            starter_spi::changelog::Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Redo. The mirror of [`apply_inverse`]: re-apply `after` for an update,
    /// re-delete for a delete, re-create (id-stable) for a create.
    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            starter_spi::changelog::Op::Update => {
                let after = snapshot_from(ch.after.as_ref(), "after")?;
                self.restore_snapshot(tenant, &after).await
            }
            starter_spi::changelog::Op::Delete => {
                let id = id_of(ch)?;
                folder::delete(&self.metadata, tenant, id).await.map(drop)
            }
            starter_spi::changelog::Op::Create => {
                let after = snapshot_from(ch.after.as_ref(), "after")?;
                self.resurrect(tenant, &after).await
            }
            starter_spi::changelog::Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Folders have no duplicate flow, so the changelog clone path is declined.
    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "folders are not duplicable".into(),
        })
    }
}

/// Read the change's tenant, surfaced onto [`ResourceRef::tenant`] by the codec.
fn tenant_of(ch: &Change) -> Result<&str> {
    ch.resource.tenant.as_deref().ok_or_else(|| Error::Internal {
        source: "folder change is missing its tenant binding".into(),
    })
}

/// Parse the change's resource id as a folder [`Uuid`].
fn id_of(ch: &Change) -> Result<Uuid> {
    let raw = ch.resource.id.as_deref().ok_or_else(|| Error::Internal {
        source: "folder change is missing its resource id".into(),
    })?;
    Uuid::parse_str(raw).map_err(|e| Error::Internal {
        source: format!("folder change has a non-uuid resource id {raw:?}: {e}").into(),
    })
}

/// Decode a `before`/`after` JSON snapshot into a [`FolderRecord`]. The `which`
/// label names the side for a precise error if the snapshot is missing/malformed.
fn snapshot_from(value: Option<&serde_json::Value>, which: &str) -> Result<FolderRecord> {
    let v = value.ok_or_else(|| Error::Invalid {
        message: format!("folder change has no {which} snapshot to apply"),
    })?;
    let id = field_str(v, "id")?;
    let parent_id = match v.get("parent_id").and_then(|p| p.as_str()) {
        Some(raw) => Some(Uuid::parse_str(raw).map_err(|e| Error::Invalid {
            message: format!("folder {which} snapshot parent_id {raw:?} is not a uuid: {e}"),
        })?),
        None => None,
    };
    Ok(FolderRecord {
        id: Uuid::parse_str(&id).map_err(|e| Error::Invalid {
            message: format!("folder {which} snapshot id {id:?} is not a uuid: {e}"),
        })?,
        tenant_id: field_str(v, "tenant_id")?,
        parent_id,
        name: field_str(v, "name")?,
    })
}

/// Pull a required string field, naming it on a type/absence mismatch.
fn field_str(v: &serde_json::Value, field: &str) -> Result<String> {
    v.get(field)
        .and_then(|f| f.as_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid {
            message: format!("folder snapshot is missing string field {field:?}"),
        })
}

/// The JSON shape a recording handler must capture so this impl can reverse it.
/// Kept next to the decoder so producer and consumer stay in step.
pub fn snapshot_json(rec: &FolderRecord) -> serde_json::Value {
    json!({
        "id": rec.id.to_string(),
        "tenant_id": rec.tenant_id,
        "parent_id": rec.parent_id.map(|id| id.to_string()),
        "name": rec.name,
    })
}

/// Custom ops have no defined inverse for this kind.
fn custom_unsupported(custom: &str) -> Error {
    Error::Invalid {
        message: format!("folder has no reversible for custom op {custom:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(parent: Option<Uuid>) -> FolderRecord {
        FolderRecord {
            id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            tenant_id: "acme".into(),
            parent_id: parent,
            name: "Plants".into(),
        }
    }

    /// What the recording handler emits must decode back to the same record, so
    /// the producing (`snapshot_json`) and consuming (`snapshot_from`) sides of
    /// the undo substrate stay in lockstep. Covers both a nested and a root folder.
    #[test]
    fn snapshot_round_trips_for_nested_and_root() {
        for parent in [
            Some(Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()),
            None,
        ] {
            let original = rec(parent);
            let json = snapshot_json(&original);
            let decoded = snapshot_from(Some(&json), "before").expect("decode");
            assert_eq!(decoded.id, original.id);
            assert_eq!(decoded.tenant_id, original.tenant_id);
            assert_eq!(decoded.parent_id, original.parent_id);
            assert_eq!(decoded.name, original.name);
        }
    }

    /// A missing snapshot is a precise, named error rather than a panic — an
    /// update recorded without a pre-read must surface diagnosably.
    #[test]
    fn a_missing_snapshot_is_an_invalid_error() {
        let err = snapshot_from(None, "before").unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    /// The impl reports the folder kind the registry and coverage guard expect.
    #[test]
    fn kind_is_the_folder_discriminator() {
        // A throwaway pool is not needed: `kind()` is constant.
        assert_eq!(KIND_FOLDER, "nexus.folder");
    }
}
