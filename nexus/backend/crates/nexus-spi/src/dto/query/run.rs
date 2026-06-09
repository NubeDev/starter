//! `POST /datasources/:id/query` — run one SQL statement, get rows back.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::shared::{ColumnSchema, QueryStats};

/// Body of a one-shot query. The caller supplies only the SQL; every safety
/// bound (read-only role, statement timeout, forced `LIMIT`, row/byte caps) is
/// applied server-side and is deliberately *not* expressible here — a client
/// cannot raise its own limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// The SQL to run against the datasource. Pushed down to the source database
    /// so `WHERE`/`LIMIT` execute there, not in memory.
    pub sql: String,
}

/// A completed query result: column schema, rows as JSON objects keyed by
/// column name, and execution stats. Rows are an array of objects rather than a
/// column-major frame because panels consume row records directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QueryResponse {
    /// Ordered column schema for the result set.
    pub columns: Vec<ColumnSchema>,
    /// One JSON object per row, keyed by column name.
    pub rows: Vec<Value>,
    /// Execution metadata, including the `truncated` cap signal.
    pub stats: QueryStats,
}
