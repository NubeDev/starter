//! `POST /datasources/test` — probe connectivity for raw config, and
//! `POST /datasources/:id/test` — probe a saved datasource.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::shared::DatasourceKind;

/// Body for a *pre-save* connection probe. Carries the same connection fields as
/// a create request, including the write-only secret, so the "Test connection"
/// affordance works before the datasource is persisted (and before a secret is
/// sealed). The secret is used transiently to connect and never stored or echoed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TestConnectionRequest {
    /// Which connector to probe.
    pub kind: DatasourceKind,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    /// Write-only secret used only to open the probe connection.
    pub password: String,
}

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
