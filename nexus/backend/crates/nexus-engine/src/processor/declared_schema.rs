//! Parse a `json_to_arrow` declared schema from flow config into an Arrow
//! [`Schema`].
//!
//! Arrow's own `Schema` is not serde-deserialisable without enabling a feature
//! on the arrow dependency, and a flow author should not have to write Arrow's
//! internal JSON anyway. The declared schema is therefore a small explicit field
//! list — `{ "fields": [{ "name": "temp_c", "type": "float" }, … ] }` — over a
//! closed set of primitive types. Declaring a schema pins the stream's columns
//! up front (preferred for a warehouse sink) instead of inferring them from the
//! first batch.

use std::sync::Arc;

use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef, TimeUnit};
use serde::Deserialize;
use serde_json::Value;

use crate::core::{EngineError, EngineResult};

#[derive(Debug, Deserialize)]
struct DeclaredSchema {
    fields: Vec<DeclaredField>,
}

#[derive(Debug, Deserialize)]
struct DeclaredField {
    name: String,
    #[serde(rename = "type")]
    field_type: DeclaredType,
    /// Whether the column may contain nulls. Defaults to nullable, the safe
    /// choice for ingested device data where a field may be absent.
    #[serde(default = "default_nullable")]
    nullable: bool,
}

fn default_nullable() -> bool {
    true
}

/// The closed set of primitive column types a flow may declare. Kept coarse on
/// purpose — a flow shapes device telemetry, not the full Arrow type lattice.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DeclaredType {
    Bool,
    Int,
    Float,
    String,
    /// An RFC3339 timestamp string, parsed to a microsecond timestamp.
    Timestamp,
}

impl DeclaredType {
    fn arrow(self) -> DataType {
        match self {
            DeclaredType::Bool => DataType::Boolean,
            DeclaredType::Int => DataType::Int64,
            DeclaredType::Float => DataType::Float64,
            DeclaredType::String => DataType::Utf8,
            DeclaredType::Timestamp => DataType::Timestamp(TimeUnit::Microsecond, None),
        }
    }
}

/// Parse the optional `schema` field of a `json_to_arrow` config. Returns `None`
/// when no schema is declared (infer-on-first-batch applies), or
/// [`EngineError::Build`] on a malformed declaration.
pub fn parse(config: &Value) -> EngineResult<Option<SchemaRef>> {
    let Some(value) = config.get("schema") else {
        return Ok(None);
    };
    let declared: DeclaredSchema = serde_json::from_value(value.clone())
        .map_err(|e| EngineError::Build(format!("invalid json_to_arrow schema: {e}")))?;
    if declared.fields.is_empty() {
        return Err(EngineError::Build(
            "json_to_arrow schema declares no fields".into(),
        ));
    }
    let fields: Vec<Field> = declared
        .fields
        .into_iter()
        .map(|f| Field::new(f.name, f.field_type.arrow(), f.nullable))
        .collect();
    Ok(Some(Arc::new(Schema::new(fields))))
}
