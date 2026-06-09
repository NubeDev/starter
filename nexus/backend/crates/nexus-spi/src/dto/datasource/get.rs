//! `GET /datasources/:id` — one datasource with its redacted connection.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::shared::{DatasourceKind, RedactedConnection};

/// Full datasource view. The connection is redacted — host/port/database/user
/// only, never the secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DatasourceDetail {
    /// Immutable datasource id.
    pub id: Uuid,
    pub name: String,
    pub kind: DatasourceKind,
    pub connection: RedactedConnection,
}
