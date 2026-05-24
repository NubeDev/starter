//! Boot-time tool registry.
//!
//! Returns the list of `Tool` instances the rubix-agent advertises
//! to MCP / REST callers. One entry per `rubix.<goal>.<verb>` verb
//! the binary has wired. The transport layer reads this once at
//! startup and serves the registry's contents from then on. See
//! [docs/design/tools/](../../docs/design/tools/README.md).

use std::sync::Arc;

use rubix_tools::dashboard::assistant::DashboardAssistantStub;
use rubix_tools::system::alert_send::AlertSendTool;
use rubix_tools::system::db::DbTool;
use rubix_tools::system::disk::DiskTool;
use rubix_tools::system::flow_errors::FlowErrorsTool;
use starter_spi::tool::Tool;
use starter_store_clickhouse::ChClient;

/// Build the tool registry the agent serves at boot.
///
/// `ch` carries a live ClickHouse client used by tools that write
/// history rows on success (currently just [`DiskTool`]). Passing
/// `None` keeps the verbs runnable without a warehouse — the
/// history write is skipped silently. Order matches the canonical
/// id order so the boot log and any generated OpenAPI / MCP
/// listing are stable across restarts.
pub fn build_tool_registry(
    ch: Option<Arc<ChClient>>,
    insights_disk_threshold: u8,
) -> Vec<Arc<dyn Tool>> {
    let mut disk = DiskTool::default().with_insights_threshold(insights_disk_threshold);
    if let Some(client) = ch {
        disk = disk.with_history(client);
    }
    vec![
        Arc::new(disk),
        Arc::new(DbTool),
        Arc::new(FlowErrorsTool::default()),
        Arc::new(AlertSendTool),
        // Goal 1 is still deferred — this stub surfaces so the flow
        // YAML `flows/dashboard-assistant.yaml` can dispatch its
        // primary tool to a Diagnostic with code
        // `rubix.goal.not_wired`. See
        // `rubix-tools/src/dashboard/assistant.rs` for unblock
        // criteria.
        Arc::new(DashboardAssistantStub),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_disk_tool() {
        let names: Vec<String> = build_tool_registry(None, 90)
            .iter()
            .map(|t| t.definition().name)
            .collect();
        assert!(names.contains(&"rubix.system.disk".to_owned()));
    }

    #[test]
    fn registry_contains_every_wired_system_tool() {
        let names: Vec<String> = build_tool_registry(None, 90)
            .iter()
            .map(|t| t.definition().name)
            .collect();
        for expected in [
            "rubix.system.disk",
            "rubix.system.db",
            "rubix.system.flow_errors",
            "rubix.alert.send",
        ] {
            assert!(
                names.contains(&expected.to_owned()),
                "registry missing {expected}",
            );
        }
    }
}
