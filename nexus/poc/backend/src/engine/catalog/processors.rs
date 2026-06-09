//! Catalog of built-in ArkFlow processor types and their key fields.

use crate::dto::catalog::{ComponentKind, Field, FieldKind::*};

/// The processors a user can chain in the pipeline.
pub fn list() -> Vec<ComponentKind> {
    vec![
        ComponentKind::new(
            "json_to_arrow",
            "JSON → Arrow",
            "Parse JSON payloads into Arrow record batches. Usually the first step.",
            vec![],
        ),
        ComponentKind::new(
            "arrow_to_json",
            "Arrow → JSON",
            "Serialize Arrow batches back to JSON. Usually the last step.",
            vec![],
        ),
        ComponentKind::new(
            "sql",
            "SQL",
            "Transform the in-flight batch with DataFusion SQL over the `flow` table.",
            vec![
                Field::new("query", Code, true)
                    .with("SELECT * FROM flow WHERE value >= 10", "DataFusion SQL run on each batch."),
                Field::new("table_name", Text, false).with("flow", "Table name the batch is exposed as."),
            ],
        ),
        ComponentKind::new(
            "vrl",
            "VRL",
            "Reshape records with a Vector Remap Language program.",
            vec![Field::new("statement", Code, true)
                .with(".doubled = .value * 2", "VRL program applied per record.")],
        ),
        ComponentKind::new(
            "batch",
            "Batch",
            "Re-batch the stream by count or timeout.",
            vec![
                Field::new("count", Number, false).with("100", "Rows per output batch."),
                Field::new("timeout_ms", Number, false).with("1000", "Flush after this many ms."),
            ],
        ),
    ]
}
