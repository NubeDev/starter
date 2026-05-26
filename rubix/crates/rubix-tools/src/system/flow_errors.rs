//! `rubix.system.flow_errors` — tool dispatch.
//!
//! Counts errored flow executions within a look-back window and
//! returns a small sample. v0 reads from an in-process registry
//! handle the tool is constructed with; the audit-projection
//! variant arrives with the persistence wiring. See
//! [docs/design/audit/](../../../../docs/design/audit/README.md).

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rubix_spi::dto::system::flow_errors::{
    FlowErrorSample, FlowErrorsRequest, FlowErrorsResponse, ERROR_THRESHOLD, WARN_THRESHOLD,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

/// Default look-back window applied when the request omits one.
pub const DEFAULT_WINDOW_SECS: u32 = 3_600;

/// Maximum number of error samples carried back in the response.
pub const MAX_SAMPLES: usize = 10;

/// In-process buffer of recent flow errors. The flow runtime pushes
/// entries; the tool reads them under a look-back window.
#[derive(Debug, Default)]
pub struct FlowErrorRegistry {
    entries: Mutex<Vec<FlowErrorSample>>,
}

impl FlowErrorRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one error sighting. Older entries are kept; the tool
    /// is responsible for filtering by window.
    pub fn record(&self, sample: FlowErrorSample) {
        if let Ok(mut guard) = self.entries.lock() {
            guard.push(sample);
        }
    }

    /// Snapshot all entries within `window_secs` of `now_ms`.
    fn snapshot(&self, now_ms: i64, window_secs: u32) -> Vec<FlowErrorSample> {
        let cutoff = now_ms.saturating_sub(i64::from(window_secs).saturating_mul(1_000));
        let guard = match self.entries.lock() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };
        guard
            .iter()
            .filter(|s| s.at_ms >= cutoff)
            .cloned()
            .collect()
    }
}

/// Concrete `Tool` impl for `rubix.system.flow_errors`.
#[derive(Debug, Clone)]
pub struct FlowErrorsTool {
    registry: Arc<FlowErrorRegistry>,
}

impl FlowErrorsTool {
    /// Construct the tool bound to a shared error registry.
    pub fn new(registry: Arc<FlowErrorRegistry>) -> Self {
        Self { registry }
    }
}

impl Default for FlowErrorsTool {
    fn default() -> Self {
        Self::new(Arc::new(FlowErrorRegistry::new()))
    }
}

#[async_trait]
impl Tool for FlowErrorsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.system.flow_errors".to_owned(),
            description: "Report how many flow executions have errored in a recent time window."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "window_secs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Look-back window in seconds. Defaults to 3600."
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: FlowErrorsRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("FlowErrorsRequest: {e}"),
        })?;
        let resp = probe(&self.registry, req)?;
        serde_json::to_value(resp).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

/// Pure dispatch — separated so integration tests can call it
/// without spinning up the MCP transport.
pub fn probe(registry: &FlowErrorRegistry, req: FlowErrorsRequest) -> Result<FlowErrorsResponse> {
    let probed_at_ms = now_epoch_ms();
    let window_secs = req.window_secs.unwrap_or(DEFAULT_WINDOW_SECS);

    let mut samples = registry.snapshot(probed_at_ms, window_secs);
    samples.sort_by_key(|s| s.at_ms);
    let error_count = u32::try_from(samples.len()).unwrap_or(u32::MAX);
    if samples.len() > MAX_SAMPLES {
        let drop = samples.len() - MAX_SAMPLES;
        samples.drain(0..drop);
    }

    let code = severity_key(error_count);
    let summary = Diagnostic::new(code)
        .with_param("count", DiagnosticParam::I64(i64::from(error_count)))
        .with_param("window", DiagnosticParam::I64(i64::from(window_secs)))
        .with_param("at", DiagnosticParam::Timestamp(probed_at_ms));

    Ok(FlowErrorsResponse {
        summary,
        window_secs,
        error_count,
        samples,
        probed_at_ms,
    })
}

fn severity_key(count: u32) -> MessageKey {
    let raw = if count >= ERROR_THRESHOLD {
        "rubix.system.flow_errors.error"
    } else if count >= WARN_THRESHOLD {
        "rubix.system.flow_errors.warn"
    } else {
        "rubix.system.flow_errors.ok"
    };
    MessageKey::parse(raw).expect("hard-coded keys parse")
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_key_picks_ok_for_zero() {
        assert_eq!(severity_key(0).as_str(), "rubix.system.flow_errors.ok");
    }

    #[test]
    fn severity_key_picks_warn_in_warn_band() {
        assert_eq!(severity_key(1).as_str(), "rubix.system.flow_errors.warn");
        assert_eq!(severity_key(9).as_str(), "rubix.system.flow_errors.warn");
    }

    #[test]
    fn severity_key_picks_error_at_threshold() {
        assert_eq!(severity_key(10).as_str(), "rubix.system.flow_errors.error");
        assert_eq!(
            severity_key(1_000).as_str(),
            "rubix.system.flow_errors.error"
        );
    }

    #[tokio::test]
    async fn probe_returns_ok_on_empty_registry() {
        let registry = FlowErrorRegistry::new();
        let resp = probe(&registry, FlowErrorsRequest::default()).expect("probe succeeds");
        assert_eq!(resp.error_count, 0);
        assert!(resp.samples.is_empty());
        assert_eq!(resp.window_secs, DEFAULT_WINDOW_SECS);
        assert_eq!(resp.summary.code.as_str(), "rubix.system.flow_errors.ok");
        assert!(resp.summary.params.contains_key("at"));
    }

    #[tokio::test]
    async fn probe_filters_entries_outside_window() {
        let registry = FlowErrorRegistry::new();
        registry.record(FlowErrorSample {
            flow_id: "f1".to_owned(),
            message: "boom".to_owned(),
            at_ms: 1, // far in the past
        });
        let resp = probe(
            &registry,
            FlowErrorsRequest {
                window_secs: Some(60),
            },
        )
        .expect("probe succeeds");
        assert_eq!(resp.error_count, 0);
    }

    #[tokio::test]
    async fn probe_truncates_samples_to_max() {
        let registry = FlowErrorRegistry::new();
        let now = now_epoch_ms();
        for i in 0..(MAX_SAMPLES as i64 + 5) {
            registry.record(FlowErrorSample {
                flow_id: format!("f{i}"),
                message: "boom".to_owned(),
                at_ms: now - i,
            });
        }
        let resp = probe(&registry, FlowErrorsRequest::default()).expect("probe succeeds");
        assert_eq!(resp.samples.len(), MAX_SAMPLES);
        assert!(resp.error_count as usize >= MAX_SAMPLES);
    }
}
