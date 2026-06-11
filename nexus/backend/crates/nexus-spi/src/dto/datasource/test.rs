//! `POST /datasources/test` — probe connectivity for raw config, and
//! `POST /datasources/:id/test` — probe a saved datasource.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::shared::DatasourceKind;

/// Body for a *pre-save* connection probe. The flat connection fields carry a
/// credentialed SQL connector's parameters (postgres), including the write-only
/// secret used transiently to connect and never stored or echoed. Stream
/// connectors (`mqtt`/`zenoh`) supply their non-SQL parameters in [`config`] —
/// the same `{endpoints, key_expr, …}` shape their datasource-kind config schema
/// declares — so one probe endpoint serves every connector. The SQL fields are
/// optional so a stream probe need not send placeholder host/port values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TestConnectionRequest {
    /// Which connector to probe.
    pub kind: DatasourceKind,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    /// Write-only secret used only to open the probe connection (SQL connectors).
    #[serde(default)]
    pub password: Option<String>,
    /// Per-kind config for non-SQL connectors (`{endpoints, mode, …}` for zenoh,
    /// `{host, port, client_id, …}` for mqtt). Ignored for postgres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,
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
