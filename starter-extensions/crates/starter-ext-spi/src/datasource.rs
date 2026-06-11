//! Datasource-access capability — shared wire types (WS-17 Wave B).
//!
//! The `datasource` capability lets an extension run full CRUD against a
//! configured host datasource through the `datasource.*` host methods, scoped
//! to the caller's tenant exactly like the human `POST /datasources/{id}/query`
//! route:
//!
//! - [`DatasourceQueryRequest`] — `datasource.query`: a read (SELECT). Runs the
//!   same guarded, read-only, tenant-bound path the human query route uses.
//! - [`DatasourceExecuteRequest`] — `datasource.execute`: a write
//!   (INSERT/UPDATE/DELETE/DDL). The host bounds it by the ownership-prefix rule
//!   (a CREATE must target `<ext>__<table>`) and the operator `allow_foreign_tables`
//!   grant for non-owned tables.
//!
//! These carry the same opaque [`Row`](crate::warehouse::Row) as the warehouse
//! types. The capability *handle* lives in
//! [`starter-ext-sdk::ctx::DatasourceHandle`] — this crate is zero-runtime-logic
//! (SCOPE R2), so execution lives in the host integration crate (`nexus-api`).

use serde::{Deserialize, Serialize};

use crate::warehouse::Row;

/// Wire payload for `datasource.query` — a read against a named datasource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasourceQueryRequest {
    /// The datasource id (UUID string) to run against. Must be in the
    /// extension's `datasource` grant.
    pub datasource_id: String,
    /// The read SQL. Bound read-only by the host; `$caller_tenant_id` /
    /// `$caller_team_ids` host tokens are available exactly as in the human
    /// query path.
    pub sql: String,
    /// Optional positional parameters bound as `$1`, `$2`, … in `sql`.
    #[serde(default)]
    pub params: Vec<serde_json::Value>,
}

/// Wire response for `datasource.query`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasourceQueryResponse {
    /// The result rows, one opaque JSON object each.
    #[serde(default)]
    pub rows: Vec<Row>,
}

/// Wire payload for `datasource.execute` — a write/DDL against a named
/// datasource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasourceExecuteRequest {
    /// The datasource id (UUID string) to run against. Must be in the
    /// extension's `datasource` grant.
    pub datasource_id: String,
    /// The write/DDL statement. A `CREATE TABLE` must target an
    /// `<extension_id>__<table>` (the ownership prefix); CRUD against a
    /// non-owned table requires the `allow_foreign_tables` grant.
    pub statement: String,
    /// Optional positional parameters bound as `$1`, `$2`, … in `statement`.
    #[serde(default)]
    pub params: Vec<serde_json::Value>,
}

/// Wire response for `datasource.execute`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasourceExecuteResponse {
    /// Rows the engine reported as affected by the statement (0 for DDL).
    pub rows_affected: u64,
}
