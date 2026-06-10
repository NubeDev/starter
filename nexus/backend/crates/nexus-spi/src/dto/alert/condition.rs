//! The wire shape of one alert-rule condition.
//!
//! A rule is a list of conditions combined with AND/OR. Each condition runs a
//! query, reduces its rows to one value, and compares that value to a threshold.
//! This is the serialised form carried in the rule's `conditions` array and
//! persisted as jsonb; the evaluator's pure combination logic lives in the API
//! crate, but the shape is a contract so it lives here.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One condition of an alert rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AlertCondition {
    /// The query whose reduced first column is the evaluated value.
    pub query: String,
    /// How the query's rows collapse to one value: last|min|max|avg|sum|count.
    #[serde(default = "default_reducer")]
    pub reducer: String,
    /// Comparison operator: gt|gte|lt|lte|eq|ne.
    pub op: String,
    pub threshold: f64,
}

fn default_reducer() -> String {
    "last".to_string()
}
