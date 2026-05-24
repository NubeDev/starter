//! `rubix.system.disk` — tool dispatch.
//!
//! Wraps the cross-consumer disk probe in
//! [`starter_tool_sysdiag::disk_usage`] with rubix's severity
//! thresholds and `MessageKey` taxonomy. The probe itself lives
//! upstream so any starter consumer (a maintenance CLI, another
//! agent) reuses it. See [docs/design/tools/](../../../../docs/design/tools/README.md).

use std::path::PathBuf;
use std::sync::Arc;
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
use starter_store_clickhouse::ChClient;
use starter_tool_sysdiag::{disk_usage, DiskProbeError};
use uuid::Uuid;

use crate::system::alert_send;

/// Threshold the v0 insights gate uses to decide whether to fire
/// `rubix.alert.send`. Lives in this file because the gate is a
/// hardcoded `if` for v0; the moment a second rule appears the
/// rule lifts into `starter-insights::RuleRegistry` and this
/// constant migrates with it. See
/// [docs/design/insights/](../../../../docs/design/insights/README.md).
pub const INSIGHTS_DISK_ALERT_THRESHOLD: u8 = 90;

/// Concrete `Tool` impl for `rubix.system.disk`. Optionally holds
/// a `ChClient` for the history-row write and a tenant id for the
/// per-row isolation column; both default to `None` / `Uuid::nil()`
/// so the in-process unit tests do not need a live ClickHouse.
#[derive(Clone)]
pub struct DiskTool {
    history: Option<Arc<ChClient>>,
    tenant_id: Uuid,
    host: Option<String>,
    insights_threshold: u8,
}

impl Default for DiskTool {
    fn default() -> Self {
        Self {
            history: None,
            tenant_id: Uuid::nil(),
            host: None,
            insights_threshold: INSIGHTS_DISK_ALERT_THRESHOLD,
        }
    }
}

impl std::fmt::Debug for DiskTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskTool")
            .field("history", &self.history.is_some())
            .field("tenant_id", &self.tenant_id)
            .field("host", &self.host)
            .finish()
    }
}

impl DiskTool {
    /// Attach a `ChClient` so successful probes write one row to
    /// `system_disk_history`. Without a client the verb still
    /// runs; the history insert is skipped silently (the in-process
    /// insights test path takes this branch).
    pub fn with_history(mut self, client: Arc<ChClient>) -> Self {
        self.history = Some(client);
        self
    }

    /// Tenant id stamped on every history row. Per the per-row
    /// isolation choice in `docs/design/warehouse/`; the in-process
    /// insights test path passes `Uuid::nil()` (the documented
    /// sentinel for the test seam).
    pub fn with_tenant_id(mut self, tenant_id: Uuid) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    /// Host string stamped on every history row. Defaults to
    /// `$HOSTNAME` or `"localhost"`; tests pin it for determinism.
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Override the percent-used threshold above which the
    /// post-dispatch insights gate fires `rubix.alert.send`.
    /// Defaults to [`INSIGHTS_DISK_ALERT_THRESHOLD`]; the agent
    /// binary threads `cfg.insights.disk_warn_threshold` through
    /// [`crate::system::disk::DiskTool::with_insights_threshold`] so
    /// operators can tune the gate without recompiling, and so the
    /// rubix-agent alert-path integration test can drop the
    /// threshold to a value a synthetic 60%-used probe will cross.
    pub fn with_insights_threshold(mut self, threshold: u8) -> Self {
        self.insights_threshold = threshold;
        self
    }

    fn host_str(&self) -> String {
        self.host.clone().unwrap_or_else(|| {
            std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_owned())
        })
    }
}

#[async_trait]
impl Tool for DiskTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.system.disk".to_owned(),
            description:
                "Report disk usage for a filesystem mount point on the rubix-agent host."
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
        let response = probe(req)?;

        // Write one row per probe; skipped when no ChClient is
        // bound. The history table is created by
        // `rubix/0002_history/up.sql` via the shared
        // `starter-store-clickhouse::MigrationRunner`.
        if let Some(client) = &self.history {
            write_history(client, self.tenant_id, &self.host_str(), &response).await?;
        }

        run_insights_gate(&response, self.insights_threshold).await?;

        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

/// Hardcoded v0 insights gate. Pure dispatch — separated from
/// [`Tool::invoke`] so integration tests can run it on synthetic
/// `DiskUsageResponse` values without standing up a real probe
/// (the probe reads the host filesystem and cannot easily be made
/// to report 95% used on demand). Returns `true` when the gate
/// fired an alert.
pub async fn run_insights_gate(
    response: &DiskUsageResponse,
    threshold: u8,
) -> Result<bool> {
    // The literal `> threshold` is the v0 rule; the constant
    // [`INSIGHTS_DISK_ALERT_THRESHOLD`] is the configured default
    // the agent threads in via `cfg.insights.disk_warn_threshold`.
    // TODO(upstream: rule.rhai migration) — promote to starter-insights::RuleRegistry once a second rule appears.
    if response.percent_used > threshold {
        alert_send::dispatch(AlertSeverity::Error, alert_diagnostic(response)).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Build the diagnostic the post-dispatch hook forwards to
/// `alert_send::dispatch`. Keyed off `rubix.system.disk.full` so the
/// alert sink renders the same message as the probe summary; param
/// names match the catalogue entries.
fn alert_diagnostic(response: &DiskUsageResponse) -> Diagnostic {
    Diagnostic::new(
        MessageKey::parse("rubix.system.disk.full").expect("hard-coded key parses"),
    )
    .with_param(
        "percent",
        DiagnosticParam::I64(i64::from(response.percent_used)),
    )
    .with_param("free", DiagnosticParam::I64(response.free_bytes as i64))
    .with_param("at", DiagnosticParam::Timestamp(response.probed_at_ms))
}

/// Insert one row into `system_disk_history`. The columns mirror
/// the migration in `rubix/0002_history/up.sql` exactly; the
/// `tenant_id` carries the per-row isolation discriminator.
async fn write_history(
    client: &ChClient,
    tenant_id: Uuid,
    host: &str,
    response: &DiskUsageResponse,
) -> Result<()> {
    // The official `clickhouse` crate quotes strings with
    // single-quote escaping; UUIDs round-trip as their canonical
    // hyphenated form via toUUID(). Keeping the row inline avoids
    // standing up a typed `Row` for one column set — when the
    // history table grows, the typed insert lifts into
    // `starter-store-clickhouse::store::system_disk_history` and
    // this function becomes a one-line forward.
    let sql = history_insert_sql(tenant_id, host, response);
    client
        .inner()
        .query(&sql)
        .execute()
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;
    Ok(())
}

/// SQL the history insert issues. Split out so the unit test can
/// assert the tenant_id reaches the row literally — a regression
/// that silently sent `NULL` or zero would defeat the whole point
/// of per-row isolation.
pub(crate) fn history_insert_sql(
    tenant_id: Uuid,
    host: &str,
    response: &DiskUsageResponse,
) -> String {
    let host_escaped = host.replace('\'', "''");
    format!(
        "INSERT INTO system_disk_history \
         (tenant_id, host, percent_used, free_bytes, epoch_ms) \
         VALUES (toUUID('{tenant}'), '{host}', {pct}, {free}, {ts})",
        tenant = tenant_id,
        host = host_escaped,
        pct = response.percent_used,
        free = response.free_bytes,
        ts = response.probed_at_ms,
    )
}

/// Pure dispatch — separated so integration tests can call it
/// without spinning up the MCP transport.
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

    #[test]
    fn history_insert_sql_embeds_tenant_id_and_host() {
        let response = DiskUsageResponse {
            summary: Diagnostic::new(
                MessageKey::parse("rubix.system.disk.warn").expect("hard-coded key parses"),
            ),
            mount: "/".to_owned(),
            total_bytes: 1_000_000_000,
            free_bytes: 100_000_000,
            percent_used: 95,
            probed_at_ms: 1_700_000_000_000,
        };
        let tenant = Uuid::from_u128(0xDEAD_BEEF_DEAD_BEEF_DEAD_BEEF_DEAD_BEEF);
        let sql = history_insert_sql(tenant, "test-host", &response);

        assert!(
            sql.contains("system_disk_history"),
            "must target the rubix-owned table; got {sql}"
        );
        assert!(
            sql.contains(&format!("toUUID('{tenant}')")),
            "tenant_id must travel as a toUUID() literal so the column \
             is never silently NULL/default; got {sql}",
        );
        assert!(
            sql.contains("'test-host'"),
            "host stamp must reach the row; got {sql}"
        );
        assert!(sql.contains("95"), "percent_used must reach the row");
        assert!(
            sql.contains("100000000"),
            "free_bytes must reach the row"
        );
        assert!(
            sql.contains("1700000000000"),
            "epoch_ms must reach the row"
        );
    }

    #[test]
    fn history_insert_sql_escapes_single_quotes_in_host() {
        let response = DiskUsageResponse {
            summary: Diagnostic::new(
                MessageKey::parse("rubix.system.disk.ok").expect("hard-coded key parses"),
            ),
            mount: "/".to_owned(),
            total_bytes: 1,
            free_bytes: 1,
            percent_used: 1,
            probed_at_ms: 0,
        };
        let sql = history_insert_sql(Uuid::nil(), "evil';DROP", &response);
        assert!(
            sql.contains("'evil'';DROP'"),
            "single quotes in host must be doubled to keep the literal \
             well-formed; got {sql}",
        );
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
