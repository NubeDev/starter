//! `com.rubix.weekly-report` — stub-output verb for Goal 6.
//!
//! Goal 6 (analytics + periodic reports) is deferred past the
//! goals-2-4-3 broadening. The flow YAML
//! `flows/weekly-report.yaml` still auto-surfaces as an MCP tool
//! so the catalogue stays stable; calling it returns a single
//! `Diagnostic` with code `rubix.goal.not_wired` pointing
//! operators at the design doc.
//!
//! Unblock criteria — implement when the analytics query runner +
//! blob-backed report sink land per
//! [docs/design/reports/](../../../../docs/design/reports/README.md)
//! and the `analytics.{query,report}` verbs replace this stub.

use async_trait::async_trait;
use serde_json::{json, Value};
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

/// Stub `Tool` for the weekly-report flow. Returns
/// `rubix.goal.not_wired` on every call.
#[derive(Debug, Default)]
pub struct WeeklyReportStub;

#[async_trait]
impl Tool for WeeklyReportStub {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "com.rubix.weekly-report".to_owned(),
            description:
                "Stub output for the deferred Goal 6 weekly-report flow; \
                 returns rubix.goal.not_wired."
                    .to_owned(),
            input_schema: json!({ "type": "object" }),
        }
    }

    async fn invoke(&self, _input: Value) -> Result<Value> {
        let code = MessageKey::parse("rubix.goal.not_wired").expect("hard-coded key parses");
        let diag = Diagnostic::new(code)
            .with_param("goal", DiagnosticParam::String("weekly-report".to_owned()))
            .with_param(
                "design_doc",
                DiagnosticParam::String("docs/design/reports/README.md".to_owned()),
            );
        serde_json::to_value(json!({ "summary": diag }))
            .map_err(|e| Error::Internal { source: Box::new(e) })
    }
}
