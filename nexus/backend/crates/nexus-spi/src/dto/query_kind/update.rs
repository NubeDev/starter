//! `PUT /api/v1/query/kinds/:id` request.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Partially update a query-kind; omitted fields are left unchanged. `name` is
/// immutable — a kind is not renamed — so it is absent here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UpdateQueryKindRequest {
    #[serde(default)]
    pub sql: Option<String>,
    #[serde(default)]
    pub tables: Option<Vec<String>>,
    #[serde(default)]
    pub params_schema: Option<Value>,
    #[serde(default)]
    pub datasource_kind: Option<String>,
    #[serde(default)]
    pub datasource_binding: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}
