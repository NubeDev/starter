//! Types shared across the query-kind verbs.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// A tenant-authored query-kind in full, including its `sql`. Unlike the
/// catalogue's [`QueryKindSummary`](crate::dto::query::kinds::QueryKindSummary),
/// which hides the SQL, this is the authoring view returned to the admin who
/// owns the kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QueryKindDetail {
    pub id: Uuid,
    /// Reverse-DNS id a `QueryRequest.kind` invokes (e.g. `com.acme.foo`).
    pub name: String,
    /// The raw SQL template, bound by the shared binder at run time.
    pub sql: String,
    /// The datasource shape the kind targets (e.g. `postgres`).
    pub datasource_kind: String,
    /// Tables the kind reads.
    pub tables: Vec<String>,
    /// The kind's params JSON Schema — the contract a request's `params`
    /// validates against, and what a schema-driven form renders.
    pub params_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasource_binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
