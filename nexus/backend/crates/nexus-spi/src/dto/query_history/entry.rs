//! One recalled query run, the list wrapper, and the star toggle body.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A past query run, as the recall drawer shows it. The `sql` is the authored
/// text so re-running it reproduces the original query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct QueryHistoryEntry {
    pub id: Uuid,
    /// The datasource queried, or absent for the dev single-source path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasource_id: Option<Uuid>,
    /// The authored SQL.
    pub sql: String,
    /// When the run happened, RFC-3339.
    pub ran_at: DateTime<Utc>,
    /// Wall-clock execution time, when the run completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<i64>,
    /// Rows returned, when the run completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_count: Option<i64>,
    /// The error message, when the run failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Whether the user pinned this run.
    pub starred: bool,
}

/// The history list response, newest (and starred) first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct QueryHistoryList {
    pub entries: Vec<QueryHistoryEntry>,
}

/// Body of the star toggle: the desired pinned state for one history row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StarQueryRequest {
    pub starred: bool,
}
