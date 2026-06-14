//! `GET /datasources/kinds` — the datasource-kind catalogue (WS-08b).
//!
//! A datasource-kind is a connector type declared by manifest (WS-10 §4.1B). The
//! catalogue exposes each registered kind's descriptive surface so the UI can
//! render a schema-driven config form and label its secret fields, without the
//! frontend hard-coding a form per connector. The config schema is the JSON
//! Schema the create/test path validates against; it is informational here.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The registered datasource-kinds, name-ordered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DatasourceKindList {
    /// One entry per declared connector type.
    pub kinds: Vec<DatasourceKindSummary>,
}

/// One datasource-kind's catalogue entry: enough for the UI to render a config
/// form and know how the connector is tested, never the connector's internals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DatasourceKindSummary {
    /// The kind id a datasource record stores (e.g. `postgres`, `mqtt`).
    pub name: String,
    /// Which query surface the connector serves: `query` (request → rows) or
    /// `stream` (subscribe → events for live panels/flows).
    pub surface: String,
    /// The JSON Schema the connector's config validates against — the UI builds
    /// its form from this.
    pub config_schema: serde_json::Value,
    /// Which config fields are secrets (sealed at rest, redacted on read). The UI
    /// renders these as write-only password inputs.
    pub secret_fields: Vec<String>,
    /// How connectivity is tested before save: `query` (a probe query) or
    /// `connect` (open + close a session).
    pub test_mode: String,
    /// The SQL dialect a query connector renders time macros in; absent for a
    /// stream connector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialect: Option<String>,
    /// Optional human description for the config form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
