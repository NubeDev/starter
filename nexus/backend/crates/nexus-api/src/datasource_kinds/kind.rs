//! A loaded, validated datasource-kind: its config schema, its secret fields,
//! its test descriptor, and the metadata a config form and the connect/probe
//! path need.
//!
//! A `DatasourceKind` is immutable once loaded. It holds the JSON Schema its
//! config validates against, the names of the config properties that are secrets
//! (sealed at rest), how its connectivity is tested, and — for query connectors —
//! the SQL dialect its time macros render in.

use serde_json::Value;

use super::manifest::{Surface, TestSpec};

/// One registered datasource-kind (a connector type declared by manifest). The
/// catalogue route exposes its descriptive surface; the create/test path
/// validates a config against [`DatasourceKind::config_schema`] and consults
/// [`DatasourceKind::secret_fields`] to decide what to seal.
#[derive(Debug, Clone)]
pub struct DatasourceKind {
    /// The kind id a datasource record stores (e.g. `postgres`, `mqtt`).
    pub name: String,

    /// Whether this connector is a `POST /query` target or a live stream source.
    pub surface: Surface,

    /// The JSON Schema document the connector's config validates against
    /// (`additionalProperties: false`, defaults, min/max).
    pub config_schema: Value,

    /// Config property names that hold secrets — sealed by the envelope at rest,
    /// redacted on read, decrypted only at connect.
    pub secret_fields: Vec<String>,

    /// How connectivity is tested before save.
    pub test: TestSpec,

    /// The SQL dialect a query connector renders its time macros in; `None` for a
    /// stream connector.
    pub dialect: Option<String>,

    /// Optional human description for the config form UI.
    pub description: Option<String>,
}

impl DatasourceKind {
    /// The config property names this kind's schema declares, in document order.
    /// The lint uses this to reject a `secret_field` that is not a declared
    /// config property.
    pub fn declared_config_fields(&self) -> Vec<String> {
        self.config_schema
            .get("properties")
            .and_then(Value::as_object)
            .map(|props| props.keys().cloned().collect())
            .unwrap_or_default()
    }
}
