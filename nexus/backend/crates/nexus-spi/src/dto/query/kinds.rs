//! `GET /api/v1/query/kinds` — the catalogue of registered query-kinds (WS-10).
//!
//! The kind picker in the query editor reads this to list the declarative
//! queries a panel can invoke by name instead of pasting raw SQL. It exposes
//! only the descriptive surface (name, description, datasource shape, and the
//! params JSON Schema) — never the SQL, which stays server-side.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The registered query-kinds, name-ordered, for the editor's kind picker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QueryKindList {
    /// Every registered kind, sorted by `name`.
    pub kinds: Vec<QueryKindSummary>,
}

/// One registered query-kind's descriptive surface. The `params_schema` is the
/// kind's JSON Schema document, which a schema-driven form renders to collect
/// the named params the kind binds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QueryKindSummary {
    /// Reverse-DNS id a `QueryRequest.kind` invokes (e.g. `nexus.core.meters_list`).
    pub name: String,
    /// Human description for the picker, if the manifest declared one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The datasource shape the kind targets (e.g. `postgres`).
    pub datasource_kind: String,
    /// The kind's params JSON Schema — the contract a kind-mode request's
    /// `params` validates against, and what a schema-driven form renders.
    pub params_schema: Value,
}
