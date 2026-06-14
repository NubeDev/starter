//! `GET /datasources/:id/schema` — the datasource's tables, columns, and
//! foreign-key relations, for editor autocomplete and the schema (ER) diagram.
//! A read of the datasource's `information_schema`, never row data.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One column: its name and declared SQL type (e.g. `timestamp`, `double
/// precision`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SchemaColumn {
    pub name: String,
    pub data_type: String,
}

/// One table or view, schema-qualified, with its columns in declaration order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SchemaTable {
    /// The Postgres schema the table lives in (e.g. `public`).
    pub schema: String,
    pub name: String,
    pub columns: Vec<SchemaColumn>,
}

/// One foreign-key edge: `from_column` on `from_schema.from_table` references
/// `to_column` on `to_schema.to_table`. Both ends are schema-qualified so the
/// diagram can resolve the exact tables even when a name repeats across schemas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SchemaRelation {
    pub from_schema: String,
    pub from_table: String,
    pub from_column: String,
    pub to_schema: String,
    pub to_table: String,
    pub to_column: String,
}

/// A datasource's introspected tables and foreign-key relations, for SQL
/// autocomplete in the editor and the schema (ER) diagram. `relations` is
/// additive — a client that only needs the table list can ignore it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DatasourceSchema {
    pub tables: Vec<SchemaTable>,
    #[serde(default)]
    pub relations: Vec<SchemaRelation>,
}
