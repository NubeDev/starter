//! `GET /datasources/:id/schema` — the datasource's tables and columns, for
//! editor autocomplete. A read of the datasource's `information_schema`, never
//! row data.

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

/// A datasource's introspected tables, for SQL autocomplete in the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DatasourceSchema {
    pub tables: Vec<SchemaTable>,
}
