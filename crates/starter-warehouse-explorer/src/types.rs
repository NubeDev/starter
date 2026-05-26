//! Wire types for the explorer surface.
//!
//! These match the upstream sql-studio shape with two deliberate
//! rubix-side deltas the frontend reviver already accounts for:
//!
//!   * `Overview::size_on_disk` replaces upstream's `db_size`.
//!   * `Overview::sqlite_version` is always `null` (vestigial).
//!
//! `created` / `modified` are ISO-8601 strings (or `null`); the
//! frontend revives them to `Date`. Keep this contract — the
//! reviver in
//! `packages/starter-ui-warehouse-explorer/src/hooks/use-warehouse.ts`
//! is the canonical consumer.

use serde::{Deserialize, Serialize};

/// `{ name, count }` pair used by every "X per Y" list in
/// `Overview`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountEntry {
    pub name: String,
    pub count: i64,
}

/// `GET /overview` response shape (wire format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overview {
    pub file_name: String,
    /// Always `null` from the rubix backend — vestigial sql-studio
    /// field the frontend reviver preserves for type compatibility.
    pub sqlite_version: Option<String>,
    /// Pretty-printed database size, e.g. `"312 MB"`.
    pub size_on_disk: String,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub tables: i64,
    pub indexes: i64,
    pub triggers: i64,
    pub views: i64,
    pub row_counts: Vec<CountEntry>,
    pub column_counts: Vec<CountEntry>,
    pub index_counts: Vec<CountEntry>,
}

/// `GET /tables` response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tables {
    pub tables: Vec<CountEntry>,
}

/// `GET /tables/{name}` response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    /// Synthesised `CREATE TABLE` DDL.
    pub sql: Option<String>,
    pub row_count: i64,
    pub index_count: i64,
    pub column_count: i64,
    pub table_size: String,
}

/// `GET /tables/{name}/data?page=N` and `POST /query` shared shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// `POST /query` request body.
#[derive(Debug, Clone, Deserialize)]
pub struct QueryRequest {
    pub sql: String,
}

/// `GET /autocomplete` per-table entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteTable {
    pub table_name: String,
    pub columns: Vec<String>,
}

/// `GET /autocomplete` response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Autocomplete {
    pub tables: Vec<AutocompleteTable>,
}

/// `GET /erd` column entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErdColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
}

/// `GET /erd` table entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErdTable {
    pub name: String,
    pub columns: Vec<ErdColumn>,
}

/// `GET /erd` FK relationship entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErdRelationship {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}

/// `GET /erd` response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erd {
    pub tables: Vec<ErdTable>,
    pub relationships: Vec<ErdRelationship>,
}
