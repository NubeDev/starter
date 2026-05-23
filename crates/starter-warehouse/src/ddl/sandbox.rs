//! Sandbox DDL generator. A sandbox is a plain MergeTree table
//! (no MV, no aggregation) so the analyst can drop / add columns
//! and re-import without invalidating dependents.

use serde::{Deserialize, Serialize};

use super::{validate_ident, IdentError};

#[derive(Debug, thiserror::Error)]
pub enum DdlError {
    #[error(transparent)]
    Ident(#[from] IdentError),
    #[error("column {0:?} has unsupported type {1:?}")]
    UnsupportedColumnType(String, String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SandboxColumn {
    pub name: String,
    /// CH type: `String` / `Float64` / `Int64` / `DateTime64(3)` /
    /// `UInt8`. Anything else is rejected.
    pub r#type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SandboxSpec {
    pub name: String,
    pub ttl_days: i32,
    pub columns: Vec<SandboxColumn>,
}

pub struct SandboxDdl {
    pub table_name: String,
    pub create_table: String,
    pub drop_table: String,
}

pub fn build(spec: &SandboxSpec) -> Result<SandboxDdl, DdlError> {
    let name = validate_ident(spec.name.strip_prefix("sandbox_").unwrap_or(&spec.name))?;
    let table = format!("sandbox_{name}");

    let mut cols = String::from("  ts DateTime64(3) DEFAULT now64(3)");
    for c in &spec.columns {
        validate_ident(&c.name)?;
        match c.r#type.as_str() {
            "String" | "Float64" | "Int64" | "DateTime64(3)" | "UInt8" | "Bool" => {}
            other => {
                return Err(DdlError::UnsupportedColumnType(c.name.clone(), other.to_string()))
            }
        }
        cols.push_str(&format!(",\n  {} {}", c.name, c.r#type));
    }
    cols.push_str(",\n  tags Map(String, String) DEFAULT map()");
    cols.push_str(
        ",\n  INDEX tags_bloom tags TYPE bloom_filter GRANULARITY 1",
    );

    let create_table = format!(
        "CREATE TABLE IF NOT EXISTS {table} (\n{cols}\n) ENGINE = MergeTree\nPARTITION BY toYYYYMM(ts)\nORDER BY (ts)\nTTL ts + INTERVAL {} DAY;",
        spec.ttl_days,
    );
    Ok(SandboxDdl {
        drop_table: format!("DROP TABLE IF EXISTS {table}"),
        table_name: table,
        create_table,
    })
}
