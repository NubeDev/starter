//! `rubix.system.disk` — tool dispatch.
//!
//! Wraps the cross-consumer disk probe in
//! [`starter_tool_sysdiag::disk_usage`] with rubix's severity
//! thresholds and `MessageKey` taxonomy. The probe itself lives
//! upstream so any starter consumer (a maintenance CLI, another
//! agent) reuses it. See [docs/design/tools/](../../../../docs/design/tools/README.md).
//!
//! The legacy ClickHouse `system_disk_history` write path was
//! removed in stage 3 of `rubix/docs/proposal/warehouse-engine-swap.md`.
//! A future stage will reattach a TimescaleDB-backed history
//! writer once the warehouse capability crate is rebuilt on the
//! `tsdb` engine.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rubix_spi::dto::system::alert_send::AlertSeverity;
use rubix_spi::dto::system::disk::{
    DiskUsageRequest, DiskUsageResponse, FULL_THRESHOLD, WARN_THRESHOLD,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_tool_sysdiag::{disk_usage, DiskProbeError};
use uuid::Uuid;

use crate::system::alert_send;

/// Threshold the v0 insights gate uses to decide whether to fire
/// `rubix.alert.send`.
pub const INSIGHTS_DISK_ALERT_THRESHOLD: u8 = 90;

/// Concrete `Tool` impl for `rubix.system.disk`.
#[derive(Clone, Default)]
pub struct DiskTool {
    tenant_id: Uuid,
    host: Option<String>,
    insights_threshold: u8,
}

impl std::fmt::Debug for DiskTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskTool")
            .field("tenant_id", &self.tenant_id)
            .field("host", &self.host)
            .finish()
    }
}

impl DiskTool {
    /// New tool with default thresholds.
    pub fn new() -> Self {
        Self {
            tenant_id: Uuid::nil(),
            host: None,
            insights_threshold: INSIGHTS_DISK_ALERT_THRESHOLD,
        }
    }

    /// Tenant id stamped on every future history row (currently
    /// unused — see module docs).
    // TODO(tenant): re-thread once the tsdb-backed history writer
    // is restored.
    pub fn with_tenant_id(mut self, tenant_id: Uuid) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    /// Host string the future history row will carry.
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Override the percent-used threshold.
    pub fn with_insights_threshold(mut self, threshold: u8) -> Self {
        self.insights_threshold = threshold;
        self
    }

    fn host_str(&self) -> String {
        self.host
            .clone()
            .unwrap_or_else(|| std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_owned()))
    }
}

#[async_trait]
impl Tool for DiskTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.system.disk".to_owned(),
            description: "Report disk usage for a filesystem mount point on the rubix-agent host."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "mount": {
                        "type": "string",
                        "description": "Filesystem mount point to probe; defaults to the agent's CWD disk."
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: DiskUsageRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("DiskUsageRequest: {e}"),
        })?;
        // Touch `host_str()` so the helper is exercised regardless
        // of whether a history writer is attached.
        let _ = self.host_str();
        let response = probe(req)?;

        run_insights_gate(&response, self.insights_threshold).await?;

        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

/// Hardcoded v0 insights gate.
pub async fn run_insights_gate(response: &DiskUsageResponse, threshold: u8) -> Result<bool> {
    if response.percent_used > threshold {
        alert_send::dispatch(AlertSeverity::Error, alert_diagnostic(response)).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn alert_diagnostic(response: &DiskUsageResponse) -> Diagnostic {
    Diagnostic::new(MessageKey::parse("rubix.system.disk.full").expect("hard-coded key parses"))
        .with_param(
            "percent",
            DiagnosticParam::I64(i64::from(response.percent_used)),
        )
        .with_param("free", DiagnosticParam::I64(response.free_bytes as i64))
        .with_param("at", DiagnosticParam::Timestamp(response.probed_at_ms))
}

/// Pure dispatch.
pub fn probe(req: DiskUsageRequest) -> Result<DiskUsageResponse> {
    let target: Option<PathBuf> = req.mount.map(PathBuf::from);
    let usage = disk_usage(target.as_deref()).map_err(probe_error_to_starter)?;
    let probed_at_ms = now_epoch_ms();

    let code = severity_key(usage.percent_used);
    let summary = Diagnostic::new(code)
        .with_param(
            "percent",
            DiagnosticParam::I64(i64::from(usage.percent_used)),
        )
        .with_param("free", DiagnosticParam::I64(usage.free_bytes as i64))
        .with_param("at", DiagnosticParam::Timestamp(probed_at_ms));

    Ok(DiskUsageResponse {
        summary,
        mount: usage.mount,
        total_bytes: usage.total_bytes,
        free_bytes: usage.free_bytes,
        percent_used: usage.percent_used,
        probed_at_ms,
    })
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn probe_error_to_starter(err: DiskProbeError) -> Error {
    match err {
        DiskProbeError::CwdUnavailable { source } => Error::Internal {
            source: Box::new(source),
        },
        DiskProbeError::NoMountForTarget { target } => Error::Invalid {
            message: format!("no filesystem found that contains {}", target.display()),
        },
    }
}

fn severity_key(percent: u8) -> MessageKey {
    let raw = if percent >= FULL_THRESHOLD {
        "rubix.system.disk.full"
    } else if percent >= WARN_THRESHOLD {
        "rubix.system.disk.warn"
    } else {
        "rubix.system.disk.ok"
    };
    MessageKey::parse(raw).expect("hard-coded keys parse")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_key_picks_full_above_full_threshold() {
        assert_eq!(severity_key(95).as_str(), "rubix.system.disk.full");
        assert_eq!(severity_key(99).as_str(), "rubix.system.disk.full");
    }

    #[test]
    fn severity_key_picks_warn_in_warn_band() {
        assert_eq!(severity_key(80).as_str(), "rubix.system.disk.warn");
        assert_eq!(severity_key(94).as_str(), "rubix.system.disk.warn");
    }

    #[test]
    fn severity_key_picks_ok_below_warn() {
        assert_eq!(severity_key(0).as_str(), "rubix.system.disk.ok");
        assert_eq!(severity_key(79).as_str(), "rubix.system.disk.ok");
    }

    #[tokio::test]
    async fn probe_returns_summary_for_local_fs() {
        let resp = probe(DiskUsageRequest::default()).expect("local probe succeeds");
        assert!(resp.total_bytes > 0);
        let code = resp.summary.code.as_str();
        assert!(matches!(
            code,
            "rubix.system.disk.ok" | "rubix.system.disk.warn" | "rubix.system.disk.full"
        ));
        assert!(resp.summary.params.contains_key("percent"));
        assert!(resp.summary.params.contains_key("free"));
        assert!(resp.summary.params.contains_key("at"));
        assert!(resp.probed_at_ms > 0);
    }
}
