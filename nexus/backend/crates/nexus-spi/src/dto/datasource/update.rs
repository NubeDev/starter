//! `PUT /datasources/:id` — update a datasource.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Partial update. Every field is optional; `None` leaves the stored value
/// untouched. `password` follows the write-only rule — supplying it rotates the
/// secret, omitting it keeps the existing ciphertext.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateDatasourceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Supplying a new secret rotates it; omitting it keeps the current one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}
