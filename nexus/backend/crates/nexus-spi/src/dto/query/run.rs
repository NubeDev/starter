//! `POST /datasources/:id/query` — run one SQL statement, get rows back.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::shared::{ColumnSchema, QueryStats};
use crate::dto::insight::InsightRef;

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
    ///
    /// Defaults to empty so a **kind-mode** request (`kind` set) may omit it —
    /// the kind supplies its own SQL and `sql` is ignored. Raw-SQL mode still
    /// requires a non-empty `sql` (enforced downstream), so this default only
    /// relaxes the wire contract for kind-mode callers (e.g. an extension UI
    /// posting `{ kind, params }`), it does not allow an empty raw query.
    #[serde(default)]
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
    /// WS-10 kind-mode: the reverse-DNS id of a registered query-kind to run
    /// *instead of* `sql`. When set, the server resolves the kind, validates
    /// `params` against its schema, and binds the kind's own SQL — `sql` is
    /// ignored. When absent (the default), the request is raw-SQL mode and `sql`
    /// runs. This keeps `QueryRequest` a single additive contract (C7): kind-mode
    /// is opt-in, sql-mode stays the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// WS-10 kind-mode named params, validated host-side against the kind's JSON
    /// Schema before binding. Each value binds as a `$N` arg, never inlined.
    /// Ignored in raw-SQL mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// RW-05 federation: an alias → datasource-id map naming the inputs a
    /// cross-datasource (or file) `sql` joins over. Each alias is the SQL-visible
    /// table `ds_<alias>`; the server authorises every referenced id against the
    /// caller's tenant before planning. Absent (the default) keeps the request on
    /// the single-datasource push-down path — exactly today's behaviour — so this
    /// field is purely additive. Ignored in kind-mode.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<FederatedSourceRef>,
    /// RW-06 post-query insight: an optional Rhai transform (inline or a stored
    /// id) applied to the result frame *after* the query runs and *before* the
    /// response is serialized. Caps still apply after the transform — it can
    /// aggregate the result down, never grow it past a cap. Absent (the default)
    /// keeps the result exactly as the query produced it, so this field is purely
    /// additive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight: Option<InsightRef>,
}

/// One federated input reference: a SQL alias bound to a datasource id. The alias
/// becomes the `ds_<alias>` table the request's `sql` reads; the id is resolved
/// and tenant-authorised server-side. A file datasource (kind `parquet`/`csv`)
/// references its configured path; a `postgres` datasource references its `table`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FederatedSourceRef {
    /// SQL alias for this input — referenced as `ds_<alias>` in the statement.
    /// Restricted to a plain identifier server-side so it is a safe table name.
    pub alias: String,
    /// The datasource id (UUID string) this alias resolves to. Must be visible to
    /// the caller's tenant; an id the tenant cannot view is rejected, never read.
    pub datasource: String,
    /// For a `postgres` datasource, the remote table to read into the join. A
    /// file datasource ignores this (its path is the table).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
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
