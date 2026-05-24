//! `com.rubix.dashboard-assistant` — stub-output verb for Goal 1.
//!
//! Goal 1 (build / program dashboards via SDUI) is deferred past
//! the goals-2-4-3 broadening. The flow YAML
//! `flows/dashboard-assistant.yaml` still auto-surfaces as an MCP
//! tool so the catalogue stays stable across the surface; calling
//! it returns a single `Diagnostic` with code
//! `rubix.goal.not_wired` pointing operators at the design doc.
//!
//! Unblock criteria — implement when the SDUI page store + the
//! dashboard verbs (`dashboard.{create,update,list,page.set,duplicate}`)
//! land per
//! [docs/design/sdui/](../../../../docs/design/sdui/README.md).
//! At that point this file disappears: the real verbs replace the
//! stub and the YAML's `allowed_tools` switches to the production
//! list.

use async_trait::async_trait;
use serde_json::{json, Value};
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

/// Stub `Tool` for the dashboard-assistant flow. Returns
/// `rubix.goal.not_wired` on every call.
#[derive(Debug, Default)]
pub struct DashboardAssistantStub;

#[async_trait]
impl Tool for DashboardAssistantStub {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "com.rubix.dashboard-assistant".to_owned(),
            description:
                "Stub output for the deferred Goal 1 dashboard-assistant flow; \
                 returns rubix.goal.not_wired."
                    .to_owned(),
            input_schema: json!({ "type": "object" }),
        }
    }

    async fn invoke(&self, _input: Value) -> Result<Value> {
        let code = MessageKey::parse("rubix.goal.not_wired").expect("hard-coded key parses");
        let diag = Diagnostic::new(code)
            .with_param("goal", DiagnosticParam::String("dashboard-assistant".to_owned()))
            .with_param(
                "design_doc",
                DiagnosticParam::String("docs/design/sdui/README.md".to_owned()),
            );
        serde_json::to_value(json!({ "summary": diag }))
            .map_err(|e| Error::Internal { source: Box::new(e) })
    }
}
