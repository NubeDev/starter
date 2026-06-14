//! `POST /api/v1/query/kinds` request.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Create a tenant-authored query-kind. `name` is a reverse-DNS id (e.g.
/// `com.acme.foo`), `sql` the raw template, and `datasource_kind` the datasource
/// shape it targets. `params_schema` is the kind's JSON Schema document; it and
/// `tables` default to empty. The API lint-validates the SQL before insert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateQueryKindRequest {
    pub name: String,
    pub sql: String,
    pub datasource_kind: String,
    #[serde(default)]
    pub tables: Vec<String>,
    #[serde(default)]
    pub params_schema: Option<Value>,
    #[serde(default)]
    pub datasource_binding: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}
