//! `POST /datasources` — register a new datasource.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::shared::DatasourceKind;

/// Body for creating a datasource. SQL connectors (postgres) carry their
/// connection in the flat `host`/`port`/`database`/`user`/`password` fields;
/// non-SQL connectors (`mqtt`/`zenoh`) and file kinds (`parquet`/`csv`) supply
/// their parameters in [`config`] — the same per-kind shape their
/// datasource-kind config schema declares — so one create endpoint serves every
/// connector. The flat SQL fields are optional so a stream/file create need not
/// send placeholder host/port values. The `password` is write-only: it is
/// accepted here, envelope-encrypted at rest, and never echoed back by any read
/// endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateDatasourceRequest {
    /// Human-readable name shown in the datasource picker.
    pub name: String,
    /// Which connector to use.
    pub kind: DatasourceKind,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    /// Write-only secret (SQL connectors). Stored as ciphertext; absent from
    /// every response. Optional — file kinds have no secret.
    #[serde(default)]
    pub password: Option<String>,
    /// Per-kind config for non-SQL connectors (`{endpoints, mode, …}` for zenoh,
    /// `{host, port, client_id, …}` for mqtt, `{path, has_header}` for files).
    /// Ignored for postgres, which uses the flat fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,
}
