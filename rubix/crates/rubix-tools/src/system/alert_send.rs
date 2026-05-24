//! `rubix.alert.send` — tool dispatch.
//!
//! v0 emits the alert to the local tracing sink only; real
//! downstream channels (email / webhook / paging) arrive with the
//! alert-sink wiring tracked in
//! [docs/design/audit/](../../../../docs/design/audit/README.md).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rubix_spi::dto::system::alert_send::{
    AlertSendRequest, AlertSendResponse, AlertSeverity, MESSAGE_MAX_CHARS,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use tracing::{error, info, warn};

/// Process-wide counter incremented on every successful
/// [`dispatch`] call. Integration tests assert the post-dispatch
/// insights gate fires exactly once per threshold-crossing probe;
/// production readers should use the tracing pipeline instead.
static ALERTS_FIRED: AtomicU64 = AtomicU64::new(0);

/// Snapshot of the dispatch counter. Tests read this before and
/// after exercising the insights gate so the assertion is a delta,
/// not an absolute (other tests in the binary may have fired the
/// gate too).
pub fn dispatched_count() -> u64 {
    ALERTS_FIRED.load(Ordering::Relaxed)
}

/// Concrete `Tool` impl for `rubix.alert.send`. Holds no
/// state; each invocation emits to tracing immediately.
#[derive(Debug, Default)]
pub struct AlertSendTool;

#[async_trait]
impl Tool for AlertSendTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.alert.send".to_owned(),
            description: "Emit a single operator alert via the configured alert sink.".to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["severity", "message"],
                "properties": {
                    "severity": {
                        "type": "string",
                        "enum": ["info", "warn", "error"],
                        "description": "Severity level attached to the alert."
                    },
                    "message": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Operator-readable alert body; truncated past 1024 chars."
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: AlertSendRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("AlertSendRequest: {e}"),
            })?;
        let resp = probe(req)?;
        serde_json::to_value(resp).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

/// Pure dispatch — separated so integration tests can call it
/// without spinning up the MCP transport.
pub fn probe(req: AlertSendRequest) -> Result<AlertSendResponse> {
    if req.message.trim().is_empty() {
        return Err(Error::Invalid {
            message: "alert message must not be empty".to_owned(),
        });
    }

    let truncated: String = req.message.chars().take(MESSAGE_MAX_CHARS).collect();
    let delivered_chars = u32::try_from(truncated.chars().count()).unwrap_or(u32::MAX);

    match req.severity {
        AlertSeverity::Info => info!(target: "rubix.alert", "{truncated}"),
        AlertSeverity::Warn => warn!(target: "rubix.alert", "{truncated}"),
        AlertSeverity::Error => error!(target: "rubix.alert", "{truncated}"),
    }
    ALERTS_FIRED.fetch_add(1, Ordering::Relaxed);

    let probed_at_ms = now_epoch_ms();
    let code = MessageKey::parse("rubix.alert.send.ok").expect("hard-coded key parses");
    let summary = Diagnostic::new(code)
        .with_param("severity", DiagnosticParam::String(severity_str(req.severity).to_owned()))
        .with_param("at", DiagnosticParam::Timestamp(probed_at_ms));

    Ok(AlertSendResponse {
        summary,
        severity: req.severity,
        delivered_chars,
        probed_at_ms,
    })
}

fn severity_str(s: AlertSeverity) -> &'static str {
    match s {
        AlertSeverity::Info => "info",
        AlertSeverity::Warn => "warn",
        AlertSeverity::Error => "error",
    }
}

/// Insights-gate entry point. The disk verb's post-dispatch hook
/// calls this when a threshold-crossing probe lands; downstream
/// rule.rhai migration (see `docs/design/insights/`) replaces the
/// hardcoded `if` with a `RuleRegistry::evaluate` and reuses the
/// same [`dispatch`] callback.
///
/// The shape is `(severity, MessageKey, params)` rather than the
/// public [`AlertSendRequest`] DTO so the gate can pass through
/// the structured diagnostic the probe built without flattening it
/// to a string twice. The tracing line carries `severity`, `key`
/// and the param map as structured fields so log aggregators can
/// filter on them.
pub async fn dispatch(
    severity: AlertSeverity,
    diag: Diagnostic,
) -> Result<AlertSendResponse> {
    let probed_at_ms = now_epoch_ms();
    let key = diag.code.as_str();
    let params = serde_json::to_string(&diag.params)
        .unwrap_or_else(|_| "{}".to_owned());

    match severity {
        AlertSeverity::Info => info!(
            target: "rubix.alert",
            severity = severity_str(severity),
            key,
            params = %params,
            "rubix.alert.send fired"
        ),
        AlertSeverity::Warn => warn!(
            target: "rubix.alert",
            severity = severity_str(severity),
            key,
            params = %params,
            "rubix.alert.send fired"
        ),
        AlertSeverity::Error => error!(
            target: "rubix.alert",
            severity = severity_str(severity),
            key,
            params = %params,
            "rubix.alert.send fired"
        ),
    }
    ALERTS_FIRED.fetch_add(1, Ordering::Relaxed);

    let summary = Diagnostic::new(
        MessageKey::parse("rubix.alert.send.ok").expect("hard-coded key parses"),
    )
    .with_param(
        "severity",
        DiagnosticParam::String(severity_str(severity).to_owned()),
    )
    .with_param("at", DiagnosticParam::Timestamp(probed_at_ms));

    let delivered_chars = u32::try_from(params.chars().count()).unwrap_or(u32::MAX);

    Ok(AlertSendResponse {
        summary,
        severity,
        delivered_chars,
        probed_at_ms,
    })
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

    #[tokio::test]
    async fn probe_emits_info_severity() {
        let resp = probe(AlertSendRequest {
            severity: AlertSeverity::Info,
            message: "hello".to_owned(),
        })
        .expect("probe succeeds");
        assert_eq!(resp.severity, AlertSeverity::Info);
        assert_eq!(resp.delivered_chars, 5);
        assert_eq!(resp.summary.code.as_str(), "rubix.alert.send.ok");
        assert!(resp.summary.params.contains_key("at"));
    }

    #[tokio::test]
    async fn probe_truncates_long_messages() {
        let long = "x".repeat(MESSAGE_MAX_CHARS + 50);
        let resp = probe(AlertSendRequest {
            severity: AlertSeverity::Warn,
            message: long,
        })
        .expect("probe succeeds");
        assert_eq!(resp.delivered_chars as usize, MESSAGE_MAX_CHARS);
    }

    #[tokio::test]
    async fn probe_rejects_empty_message() {
        let err = probe(AlertSendRequest {
            severity: AlertSeverity::Error,
            message: "   ".to_owned(),
        })
        .expect_err("blank message rejected");
        assert!(matches!(err, Error::Invalid { .. }));
    }
}
