//! Minimal predicate set. If you find yourself wanting `LIKE`,
//! `BETWEEN`, or boolean combinators here, you probably want a
//! consumer-side query language instead — keep this module small.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single filter clause: "field equals value" or "field in values".
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Predicate {
    /// `field = value`.
    Eq {
        /// Name of the field, matched against the endpoint's schema.
        field: String,
        /// JSON-encoded value to compare against.
        value: serde_json::Value,
    },
    /// `field IN (values...)`.
    In {
        /// Name of the field.
        field: String,
        /// Values; empty means "match nothing".
        values: Vec<serde_json::Value>,
    },
}
