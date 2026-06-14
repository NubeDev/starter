//! Validate a stream config by building it — without running it.

use arkflow_core::stream::StreamConfig;
use serde_json::Value;

/// Try to deserialize and `build()` the config. Returns the build error string
/// on failure, or `None` if the config is valid.
pub fn validate(config: Value) -> Option<String> {
    let parsed: StreamConfig = match serde_json::from_value(config) {
        Ok(cfg) => cfg,
        Err(e) => return Some(format!("invalid config: {e}")),
    };
    match parsed.build() {
        Ok(_) => None,
        Err(e) => Some(e.to_string()),
    }
}
