//! The query-history record types: the input for a new run and the row read
//! back from the ledger.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// What to record after a query runs. The SQL is the *authored* text
/// (pre-binding) so a recall re-runs exactly what the user typed. `error` is
/// `Some` only on a failed run, in which case `elapsed_ms`/`row_count` are
/// whatever the runner observed (often `None`).
#[derive(Debug, Clone)]
pub struct NewQueryRun {
    /// The starter-identity subject who ran the query.
    pub user_id: String,
    /// The datasource queried, or `None` for the dev single-source path.
    pub datasource_id: Option<Uuid>,
    /// The authored SQL, pre-binding.
    pub sql: String,
    /// Wall-clock execution time, when the run completed.
    pub elapsed_ms: Option<i64>,
    /// Rows returned, when the run completed.
    pub row_count: Option<i64>,
    /// The error message, when the run failed.
    pub error: Option<String>,
}

/// One history row read back for the recall drawer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryHistoryRow {
    pub id: Uuid,
    pub user_id: String,
    pub datasource_id: Option<Uuid>,
    pub sql: String,
    pub ran_at: DateTime<Utc>,
    pub elapsed_ms: Option<i64>,
    pub row_count: Option<i64>,
    pub error: Option<String>,
    pub starred: bool,
}
