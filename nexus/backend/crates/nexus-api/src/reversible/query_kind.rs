//! [`Reversible`] for query-kinds — the snapshot impl that makes tenant-authored
//! named SQL queries undoable (WS-12 C6).
//!
//! Query-kinds are **snapshot** (small, plain-CRUD: create/delete flip existence;
//! an update swaps sql/params_schema/datasource_kind/tables/binding/description).
//! The recorded `before`/`after` carry the full [`QueryKindRecord`]. Unlike
//! datasources, a query-kind holds no secret, so delete is fully reversible:
//! undo-of-delete resurrects the row from its `before` snapshot.
//!
//! Unlike panels, a query-kind id is **not referenced anywhere else** — the
//! picker and the dispatcher both key on `name` (which is `UNIQUE(tenant_id,
//! name)`), and nothing stores a foreign reference to a query-kind id. So a
//! resurrected row may take a **fresh** id without breaking anything; the only
//! thing that keys on the id is the changelog itself. We therefore resurrect via
//! plain [`insert`](nexus_store::query_kind::insert) (there is no `insert_with_id`
//! for this kind), accepting a new id as harmless.
//!
//! The tenant is read from `ch.resource.tenant` (surfaced by the changelog codec)
//! so store calls bind the caller's tenant under RLS. The decoded
//! [`QueryKindRecord`] carries a `tenant_id` field (panels do not); it is
//! populated from that same change tenant binding.

use async_trait::async_trait;
use serde_json::json;
use sqlx::PgPool;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::{Error, Result};
use uuid::Uuid;

use nexus_store::query_kind::{self, NewQueryKind, QueryKindPatch, QueryKindRecord};

use crate::authz::KIND_QUERY_KIND;

/// Snapshot-strategy [`Reversible`] for query-kinds. Closes over the metadata
/// pool so inverses apply against the store inside a tenant transaction.
pub struct QueryKindReversible {
    metadata: PgPool,
}

impl QueryKindReversible {
    /// Build the reversible over the metadata pool (one instance, registered once
    /// at boot).
    pub fn new(metadata: PgPool) -> Self {
        Self { metadata }
    }

    /// Re-apply a full query-kind snapshot onto the live row. Used by both undo
    /// (snapshot = `before`) and redo (snapshot = `after`): a snapshot kind has no
    /// direction-specific logic beyond which side it writes. `NotFound` if the row
    /// is gone. Every restorable field is set (full snapshot), so the
    /// COALESCE-on-`None` store semantics never leave a stale field behind;
    /// `datasource_binding`/`description` are set with the nested `Some(<Option>)`
    /// so a restore can also clear them back to NULL.
    async fn restore_snapshot(&self, tenant: &str, snapshot: &QueryKindRecord) -> Result<()> {
        let patch = QueryKindPatch {
            sql: Some(snapshot.sql.clone()),
            params_schema: Some(snapshot.params_schema.clone()),
            datasource_kind: Some(snapshot.datasource_kind.clone()),
            tables: Some(snapshot.tables.clone()),
            datasource_binding: Some(snapshot.datasource_binding.clone()),
            description: Some(snapshot.description.clone()),
        };
        // update returns NotFound (not Option) when the row is gone.
        query_kind::update(&self.metadata, tenant, snapshot.id, &patch)
            .await
            .map(drop)
    }

    /// Re-create a deleted query-kind from a snapshot. Used by undo-of-delete and
    /// redo-of-create. Note: unlike panels there is no `insert_with_id`, so the
    /// resurrected row takes a **fresh** id — harmless here because query-kind ids
    /// are not referenced elsewhere (the picker and dispatcher key on `name`).
    async fn resurrect(&self, tenant: &str, snapshot: &QueryKindRecord) -> Result<()> {
        let new = NewQueryKind {
            name: snapshot.name.clone(),
            sql: snapshot.sql.clone(),
            params_schema: snapshot.params_schema.clone(),
            datasource_kind: snapshot.datasource_kind.clone(),
            tables: snapshot.tables.clone(),
            datasource_binding: snapshot.datasource_binding.clone(),
            description: snapshot.description.clone(),
        };
        query_kind::insert(&self.metadata, tenant, &new).await.map(drop)
    }
}

#[async_trait]
impl Reversible for QueryKindReversible {
    fn kind(&self) -> &'static str {
        KIND_QUERY_KIND
    }

    /// Undo. For an update, re-apply the `before` snapshot. For a create, delete
    /// the row the create produced. For a delete, resurrect the row from the
    /// `before` snapshot.
    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            Op::Update => {
                let before = snapshot_from(ch.before.as_ref(), "before", tenant)?;
                self.restore_snapshot(tenant, &before).await
            }
            Op::Create => {
                let id = id_of(ch)?;
                query_kind::delete(&self.metadata, tenant, id).await.map(drop)
            }
            Op::Delete => {
                let before = snapshot_from(ch.before.as_ref(), "before", tenant)?;
                self.resurrect(tenant, &before).await
            }
            Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Redo. The mirror of [`apply_inverse`]: re-apply `after` for an update,
    /// re-delete for a delete, re-create for a create.
    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let tenant = tenant_of(ch)?;
        match ch.op {
            Op::Update => {
                let after = snapshot_from(ch.after.as_ref(), "after", tenant)?;
                self.restore_snapshot(tenant, &after).await
            }
            Op::Delete => {
                let id = id_of(ch)?;
                query_kind::delete(&self.metadata, tenant, id).await.map(drop)
            }
            Op::Create => {
                let after = snapshot_from(ch.after.as_ref(), "after", tenant)?;
                self.resurrect(tenant, &after).await
            }
            Op::Custom(ref c) => Err(custom_unsupported(c)),
        }
    }

    /// Duplicate via the changelog clone path is not used for query-kinds — they
    /// are authored through the CRUD API, which records its own `Change`. A bare
    /// clone here would collide on the `UNIQUE(tenant_id, name)` name, so it is
    /// declined.
    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "query-kinds are created via POST /api/v1/query-kinds, \
                      not the changelog clone path"
                .into(),
        })
    }
}

/// Read the change's tenant binding (surfaced by the codec). Absent = substrate
/// bug.
fn tenant_of(ch: &Change) -> Result<&str> {
    ch.resource.tenant.as_deref().ok_or_else(|| Error::Internal {
        source: "query-kind change is missing its tenant binding".into(),
    })
}

/// Parse the change's resource id as a query-kind [`Uuid`].
fn id_of(ch: &Change) -> Result<Uuid> {
    let raw = ch.resource.id.as_deref().ok_or_else(|| Error::Internal {
        source: "query-kind change is missing its resource id".into(),
    })?;
    Uuid::parse_str(raw).map_err(|e| Error::Internal {
        source: format!("query-kind change has a non-uuid resource id {raw:?}: {e}").into(),
    })
}

/// Decode a `before`/`after` JSON snapshot into a [`QueryKindRecord`]. The `which`
/// label names the side for a precise error if the snapshot is missing or
/// malformed (e.g. an update recorded without a pre-read). `tenant_id` is not in
/// the snapshot (the change carries the tenant separately, like panels), so it is
/// populated from the change's tenant binding passed in as `tenant`.
fn snapshot_from(
    value: Option<&serde_json::Value>,
    which: &str,
    tenant: &str,
) -> Result<QueryKindRecord> {
    let v = value.ok_or_else(|| Error::Invalid {
        message: format!("query-kind change has no {which} snapshot to apply"),
    })?;
    let id = field_uuid(v, "id", which)?;
    // params_schema is the kind's opaque JSON Schema document, carried verbatim.
    let params_schema = v.get("params_schema").cloned().unwrap_or_else(|| json!({}));
    // tables is a JSON array of strings; an absent array decodes to empty.
    let tables = match v.get("tables") {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .map(|t| {
                t.as_str().map(str::to_owned).ok_or_else(|| Error::Invalid {
                    message: format!(
                        "query-kind {which} snapshot tables contains a non-string entry"
                    ),
                })
            })
            .collect::<Result<Vec<String>>>()?,
        Some(serde_json::Value::Null) | None => Vec::new(),
        Some(_) => {
            return Err(Error::Invalid {
                message: format!("query-kind {which} snapshot tables is not an array"),
            })
        }
    };
    Ok(QueryKindRecord {
        id,
        // tenant_id is not carried in the snapshot; bind it from the change.
        tenant_id: tenant.to_string(),
        name: field_str(v, "name")?,
        sql: field_str(v, "sql")?,
        params_schema,
        datasource_kind: field_str(v, "datasource_kind")?,
        tables,
        datasource_binding: field_opt_str(v, "datasource_binding"),
        description: field_opt_str(v, "description"),
    })
}

/// Pull a required string field from a snapshot object, naming it on a
/// type/absence mismatch so a malformed recording is diagnosable.
fn field_str(v: &serde_json::Value, field: &str) -> Result<String> {
    v.get(field)
        .and_then(|f| f.as_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid {
            message: format!("query-kind snapshot is missing string field {field:?}"),
        })
}

/// Pull an optional string field: `None` when the field is absent or JSON null.
fn field_opt_str(v: &serde_json::Value, field: &str) -> Option<String> {
    v.get(field).and_then(|f| f.as_str()).map(str::to_owned)
}

/// Pull a required uuid field from a snapshot object.
fn field_uuid(v: &serde_json::Value, field: &str, which: &str) -> Result<Uuid> {
    let raw = field_str(v, field)?;
    Uuid::parse_str(&raw).map_err(|e| Error::Invalid {
        message: format!("query-kind {which} snapshot {field} {raw:?} is not a uuid: {e}"),
    })
}

/// The JSON shape a recording handler must capture so this impl can reverse it.
/// Kept next to the decoder so the producing and consuming sides stay in step.
/// Exposed for the recording handlers and the coverage guard. Query-kinds carry
/// no secrets, so the whole record is recorded verbatim. `tenant_id` is omitted —
/// the change carries the tenant separately, exactly like panels.
pub fn snapshot_json(rec: &QueryKindRecord) -> serde_json::Value {
    json!({
        "id": rec.id.to_string(),
        "name": rec.name,
        "sql": rec.sql,
        "params_schema": rec.params_schema,
        "datasource_kind": rec.datasource_kind,
        "tables": rec.tables,
        "datasource_binding": rec.datasource_binding,
        "description": rec.description,
    })
}

/// Custom ops have no defined inverse for this kind.
fn custom_unsupported(custom: &str) -> Error {
    Error::Invalid {
        message: format!("query-kind has no reversible for custom op {custom:?}"),
    }
}
