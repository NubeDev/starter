//! Boot-time tool registry.
//!
//! Returns the list of `Tool` instances the rubix-agent advertises
//! to MCP / REST callers. One entry per `rubix.<goal>.<verb>` verb
//! the binary has wired. The transport layer reads this once at
//! startup and serves the registry's contents from then on. See
//! [docs/design/tools/](../../docs/design/tools/README.md).

use std::sync::Arc;

use rubix_tools::system::alert_send::AlertSendTool;
use rubix_tools::system::db::DbTool;
use rubix_tools::system::disk::DiskTool;
use rubix_tools::system::flow_errors::FlowErrorsTool;
use starter_spi::tool::Tool;

/// Build the tool registry the agent serves at boot.
///
/// Order matches the canonical id order so the boot log and any
/// generated OpenAPI / MCP listing are stable across restarts.
pub fn build_tool_registry() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(DiskTool::default()),
        Arc::new(DbTool),
        Arc::new(FlowErrorsTool::default()),
        Arc::new(AlertSendTool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_disk_tool() {
        let names: Vec<String> = build_tool_registry()
            .iter()
            .map(|t| t.definition().name)
            .collect();
        assert!(names.contains(&"rubix.system.disk".to_owned()));
    }

    #[test]
    fn registry_contains_every_wired_system_tool() {
        let names: Vec<String> = build_tool_registry()
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
