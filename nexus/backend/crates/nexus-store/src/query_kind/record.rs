//! Row and input shapes for the query-kinds store.

use serde_json::Value;
use uuid::Uuid;

/// A tenant-authored query-kind as stored. `params_schema` is the kind's JSON
/// Schema document; `tables` are the tables it reads. The API lint validated the
/// SQL before insert — the store only persists this, it does not re-validate.
#[derive(Debug, Clone)]
pub struct QueryKindRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub sql: String,
    pub params_schema: Value,
    pub datasource_kind: String,
    pub tables: Vec<String>,
    pub datasource_binding: Option<String>,
    pub description: Option<String>,
}

/// A new query-kind to insert.
#[derive(Debug, Clone)]
pub struct NewQueryKind {
    pub name: String,
    pub sql: String,
    pub params_schema: Value,
    pub datasource_kind: String,
    pub tables: Vec<String>,
    pub datasource_binding: Option<String>,
    pub description: Option<String>,
}

/// A partial update; `None` fields are left unchanged. `datasource_binding` and
/// `description` use a nested `Option` so the caller can distinguish "leave
/// unchanged" (`None`) from "clear it" (`Some(None)`). `name` is immutable and
/// so is absent.
#[derive(Debug, Clone, Default)]
pub struct QueryKindPatch {
    pub sql: Option<String>,
    pub params_schema: Option<Value>,
    pub datasource_kind: Option<String>,
    pub tables: Option<Vec<String>>,
    pub datasource_binding: Option<Option<String>>,
    pub description: Option<Option<String>>,
}
