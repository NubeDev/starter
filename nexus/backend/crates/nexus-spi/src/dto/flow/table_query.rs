//! `POST /api/v1/flows/{id}/table/query` request — query a flow's sink table.
//!
//! The flow owns its output table; this lets the Workbench query what actually
//! landed without the user resolving a datasource or retyping the table name. The
//! query runs read-only against the flow's sink connection, through the same
//! guards as `/api/v1/query`. The response is a plain [`crate::dto::query::QueryResponse`].

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Query the table a flow's `postgres`/`datasource` sink writes to. `sql` is
/// optional: when omitted the server runs a default "most recent rows" preview
/// against the sink table. The `{table}` token in `sql` expands to the flow's
/// configured table name, so a saved query need not hardcode it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
pub struct FlowTableQueryRequest {
    /// SQL to run (read-only). `{table}` expands to the flow's sink table. When
    /// omitted, the server runs its default recent-rows preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    /// Cap on returned rows for the preview; clamped to the server maximum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}
