//! Wire types for validating and running a stream config.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A stream config submitted from the builder UI (maps 1:1 to ArkFlow's
/// `StreamConfig`: `input`, `pipeline`, `output`, optional `buffer` etc).
#[derive(Debug, Clone, Deserialize)]
pub struct StreamRequest {
    #[serde(flatten)]
    pub config: Value,
}

/// Result of `cfg.build()` without running it.
#[derive(Debug, Serialize)]
pub struct ValidateResponse {
    pub ok: bool,
    pub error: Option<String>,
}

/// How long a run is allowed to execute before we cancel it.
fn default_timeout_ms() -> u64 {
    5_000
}

/// Run request: the same stream config, plus a run budget. The output is
/// replaced server-side with the in-memory collector.
#[derive(Debug, Clone, Deserialize)]
pub struct RunRequest {
    #[serde(flatten)]
    pub config: Value,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

/// Rows captured by the collector plus run metadata.
#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub ok: bool,
    pub error: Option<String>,
    pub row_count: usize,
    pub rows: Vec<Value>,
    pub cancelled: bool,
}
