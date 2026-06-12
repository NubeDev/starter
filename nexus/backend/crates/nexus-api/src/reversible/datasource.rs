//! [`Reversible`] for datasources — the secret-redacting snapshot reference impl
//! (WS-12).
//!
//! Datasources are snapshot (small, lifecycle-shaped). The recorded `before`/
//! `after` carry only the redacted [`DatasourceRecord`] — never the connection
//! secret (sealed or plain), per the WS-12 §3.2 recording contract. Because the
//! secret never enters the snapshot, an undo of an update restores every
//! non-secret field but cannot rotate the secret back; the impl applies the
//! restorable fields and leaves the secret untouched, which is the correct,
//! honest behaviour for a redacted ledger.
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

use nexus_store::datasource::{self, DatasourcePatch, DatasourceRecord, Envelope};

use crate::authz::KIND_DATASOURCE;

/// Snapshot-strategy [`Reversible`] for datasources. Closes over the metadata
/// pool and the secret [`Envelope`] (the store's update needs it to re-seal a
/// rotated secret; undo never rotates one, but the store signature requires it).
pub struct DatasourceReversible {
    metadata: PgPool,
    envelope: Envelope,
}

impl DatasourceReversible {
    /// Build the reversible over the metadata pool and envelope.
    pub fn new(metadata: PgPool, envelope: Envelope) -> Self {
        Self { metadata, envelope }
    }

    /// Re-apply the restorable (non-secret) fields of a snapshot onto the live
    /// row. The secret is deliberately absent from the snapshot (redaction
    /// contract), so it is left as-is. `NotFound` if the row is gone.
    async fn restore_snapshot(&self, tenant: &str, snapshot: &DatasourceRecord) -> Result<()> {
        let patch = DatasourcePatch {
            name: Some(snapshot.name.clone()),
            host: Some(snapshot.host.clone()),
            port: Some(snapshot.port),
            database: Some(snapshot.database.clone()),
            db_user: Some(snapshot.db_user.clone()),
            secret: None,
        };
        let updated =
            datasource::update(&self.metadata, &self.envelope, tenant, snapshot.id, &patch).await?;
        if updated {
            Ok(())
        } else {
            Err(Error::NotFound {
                what: format!("datasource {}", snapshot.id),
            })
        }
    }
}

#[async_trait]
impl Reversible for DatasourceReversible {
    fn kind(&self) -> &'static str {
        KIND_DATASOURCE
    }

    /// Undo: re-apply `before` for an update, delete the row a create produced,
    /// or refuse a delete-resurrection (needs id-stable insert — see
    /// [`resurrect_unsupported`]).
    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            Op::Update => {
                let before = snapshot_from(ch.before.as_ref(), "before")?;
                self.restore_snapshot(tenant, &before).await
            }
            Op::Create => {
                let id = id_of(ch)?;
                datasource::delete(&self.metadata, tenant, id)
                    .await
                    .map(drop)
            }
            Op::Delete => resurrect_unsupported(),
            Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Redo: the mirror of [`apply_inverse`].
    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            Op::Update => {
                let after = snapshot_from(ch.after.as_ref(), "after")?;
                self.restore_snapshot(tenant, &after).await
            }
            Op::Delete => {
                let id = id_of(ch)?;
                datasource::delete(&self.metadata, tenant, id)
                    .await
                    .map(drop)
            }
            Op::Create => resurrect_unsupported(),
            Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Duplicate needs an id + secret to seed a new row; the secret is not in the
    /// snapshot by design, so duplicate-from-changelog is unsupported here
    /// (a duplicate flow would re-prompt for the secret). See
    /// [`resurrect_unsupported`].
    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(resurrect_unsupported().unwrap_err())
    }
}

/// Read the change's tenant binding (surfaced by the codec). Absent = substrate
/// bug.
fn tenant_of(ch: &Change) -> Result<&str> {
    ch.resource
        .tenant
        .as_deref()
        .ok_or_else(|| Error::Internal {
            source: "datasource change is missing its tenant binding".into(),
        })
}

/// Parse the change's resource id as a datasource [`Uuid`].
fn id_of(ch: &Change) -> Result<Uuid> {
    let raw = ch.resource.id.as_deref().ok_or_else(|| Error::Internal {
        source: "datasource change is missing its resource id".into(),
    })?;
    Uuid::parse_str(raw).map_err(|e| Error::Internal {
        source: format!("datasource change has a non-uuid resource id {raw:?}: {e}").into(),
    })
}

/// Decode a redacted `before`/`after` snapshot into a [`DatasourceRecord`].
fn snapshot_from(value: Option<&serde_json::Value>, which: &str) -> Result<DatasourceRecord> {
    let v = value.ok_or_else(|| Error::Invalid {
        message: format!("datasource change has no {which} snapshot to apply"),
    })?;
    let id = field_str(v, "id")?;
    Ok(DatasourceRecord {
        id: Uuid::parse_str(&id).map_err(|e| Error::Invalid {
            message: format!("datasource {which} snapshot id {id:?} is not a uuid: {e}"),
        })?,
        tenant_id: field_str(v, "tenant_id")?,
        name: field_str(v, "name")?,
        kind: field_str(v, "kind")?,
        host: field_str(v, "host")?,
        port: field_i32(v, "port")?,
        database: field_str(v, "database")?,
        db_user: field_str(v, "db_user")?,
        key_version: field_i32(v, "key_version")?,
        // Optional per-kind config (file kinds); absent in a redacted snapshot
        // taken before this column existed, which is a benign None.
        config: v.get("config").cloned(),
    })
}

/// Pull a required string field, naming it on mismatch.
fn field_str(v: &serde_json::Value, field: &str) -> Result<String> {
    v.get(field)
        .and_then(|f| f.as_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid {
            message: format!("datasource snapshot is missing string field {field:?}"),
        })
}

/// Pull a required integer field, naming it on mismatch.
fn field_i32(v: &serde_json::Value, field: &str) -> Result<i32> {
    v.get(field)
        .and_then(serde_json::Value::as_i64)
        .map(|n| n as i32)
        .ok_or_else(|| Error::Invalid {
            message: format!("datasource snapshot is missing integer field {field:?}"),
        })
}

/// The redacted JSON shape a recording handler must capture. Contains **no**
/// secret material (the [`DatasourceRecord`] is already redacted), honoring the
/// WS-12 §3.2 contract. Exposed for the recording handlers and coverage guard.
pub fn snapshot_json(rec: &DatasourceRecord) -> serde_json::Value {
    json!({
        "id": rec.id.to_string(),
        "tenant_id": rec.tenant_id,
        "name": rec.name,
        "kind": rec.kind,
        "host": rec.host,
        "port": rec.port,
        "database": rec.database,
        "db_user": rec.db_user,
        "key_version": rec.key_version,
    })
}

/// Resurrecting a deleted datasource (undo-delete / redo-create / duplicate)
/// needs id-stable re-insert *and* its secret, which the redacted snapshot does
/// not carry. Honest refusal rather than a fake. Tracked in
/// `nexus/docs/scope/nextgen/sessions/TODOs.md`.
fn resurrect_unsupported() -> Result<()> {
    Err(Error::Invalid {
        message: "undo of a datasource delete (id-stable restore) is not yet supported; \
                  the secret is redacted from the snapshot and the store mints a new id \
                  (WS-08 follow-up)"
            .into(),
    })
}

/// Custom ops have no defined inverse for this kind.
fn custom_unsupported(custom: &str) -> Error {
    Error::Invalid {
        message: format!("datasource has no reversible for custom op {custom:?}"),
    }
}
