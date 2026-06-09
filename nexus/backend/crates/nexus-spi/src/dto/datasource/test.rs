//! `POST /datasources/:id/test` — probe connectivity without running a query.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Outcome of a connection probe. `ok` is the headline; on failure `message`
/// carries the redacted reason (driver errors are sanitized so they never leak
/// the connection secret).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TestDatasourceResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Round-trip latency of the probe in milliseconds, when it connected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}
