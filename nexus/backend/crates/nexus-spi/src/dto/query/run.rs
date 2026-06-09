//! `POST /datasources/:id/query` — run one SQL statement, get rows back.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::shared::{ColumnSchema, QueryStats};

/// Body of a one-shot query. The caller supplies the SQL plus the optional
/// macro context (time range, variables) the server-side binder substitutes;
/// every safety bound (read-only role, statement timeout, forced `LIMIT`,
/// row/byte caps) is applied server-side and is deliberately *not* expressible
/// here — a client cannot raise its own limits, and every supplied value is
/// *bound* into the prepared statement, never inlined.
///
/// This is the C7 contract: a single owner (WS-03) for an endpoint several
/// workstreams feed. WS-01 fills `time_range`, WS-02 fills `variables`; both
/// flow through the same binder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// The SQL to run against the datasource. May carry macros (`$__timeFilter`)
    /// and variable references (`$region`) the binder expands into bound args.
    /// Pushed down to the source database so `WHERE`/`LIMIT` execute there.
    pub sql: String,
    /// The resolved absolute time window for `$__timeFilter`/`$__timeFrom`/
    /// `$__timeTo`. Relative ranges (`now-6h`) are resolved client-side to
    /// instants before sending so the bound query and the cache key agree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_range: Option<QueryTimeRange>,
    /// The bucket width (seconds) for `$__timeGroup(col, $__interval)` and a
    /// bare `$__interval`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_secs: Option<u64>,
    /// Dashboard variable values, by name (without the leading `$`), expanded by
    /// `$var` / `${var:csv}` / `$__sqlIn(var)`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<QueryVariable>,
}

/// An absolute query window. Half-open (`from` inclusive, `to` exclusive),
/// matching the binder's `col >= from AND col < to` expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct QueryTimeRange {
    /// Inclusive lower bound, RFC-3339.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound, RFC-3339.
    pub to: DateTime<Utc>,
}

/// One dashboard variable's resolved value(s). A single-valued variable carries
/// one entry; a multi-select carries several (expanded by `$__sqlIn`/`:csv`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QueryVariable {
    /// Variable name without the `$` (e.g. `region`).
    pub name: String,
    /// One or more string values. Multiple values drive list expansion; the
    /// binder binds each as its own argument so values are always inert.
    pub values: Vec<String>,
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
