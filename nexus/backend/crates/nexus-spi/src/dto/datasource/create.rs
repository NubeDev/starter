//! `POST /datasources` — register a new datasource.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::shared::DatasourceKind;

/// Body for creating a datasource. The `password` is write-only: it is accepted
/// here, envelope-encrypted at rest, and never echoed back by any read
/// endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateDatasourceRequest {
    /// Human-readable name shown in the datasource picker.
    pub name: String,
    /// Which connector to use.
    pub kind: DatasourceKind,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    /// Write-only secret. Stored as ciphertext; absent from every response.
    pub password: String,
}
