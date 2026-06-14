//! `GET /datasources` — list datasources for the tenant.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::shared::DatasourceKind;

/// A datasource as it appears in a list — identity and kind only, no connection
/// detail. Use `GET /datasources/:id` for the redacted connection view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DatasourceSummary {
    /// Immutable datasource id (grants and panel refs key on this).
    pub id: Uuid,
    pub name: String,
    pub kind: DatasourceKind,
}
