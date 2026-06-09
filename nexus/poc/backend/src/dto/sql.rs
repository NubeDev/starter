//! Wire types for the SQL playground endpoint.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Run `query` (DataFusion SQL) over an inline array of JSON rows.
#[derive(Debug, Clone, Deserialize)]
pub struct SqlRequest {
    pub query: String,
    /// The dataset, as a JSON array of objects. Defaults to empty.
    #[serde(default)]
    pub rows: Vec<Value>,
}

#[derive(Debug, Serialize)]
pub struct SqlResponse {
    pub ok: bool,
    pub error: Option<String>,
    pub row_count: usize,
    pub rows: Vec<Value>,
}
