//! `rubix.flow_ops.lint` — tool dispatch.
//!
//! Read-only verb. Parses the YAML body through
//! `rubix_flows::parse_yaml` and `rubix_flows::convert`, surfaces
//! the resulting `LoadError` (if any) as a structured
//! [`LintDiagnostic`] with the parser's line/column annotation when
//! available, and emits `rubix.flow.linted` (no errors) or
//! `rubix.flow.lint.found_errors` (one or more). No state is
//! written. See [docs/design/flows/](../../../../docs/design/flows/README.md).

use async_trait::async_trait;
use rubix_spi::dto::flow_ops::lint::{FlowLintRequest, FlowLintResponse, LintDiagnostic};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

/// Concrete [`Tool`] for `rubix.flow_ops.lint`.
#[derive(Default)]
pub struct FlowLintTool;

impl FlowLintTool {
    /// New instance.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for FlowLintTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.flow_ops.lint".to_owned(),
            description: rubix_spi::dto::flow_ops::lint::DESCRIPTOR.purpose.to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "body_yaml": { "type": "string", "minLength": 1 }
                },
                "required": ["body_yaml"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: FlowLintRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("FlowLintRequest: {e}"),
            })?;

        let mut errors: Vec<LintDiagnostic> = Vec::new();
        match rubix_flows::parse_yaml("lint://input", req.body_yaml.as_bytes()) {
            Ok(yaml) => {
                // Run the downstream converter so semantic errors
                // (empty body, id validation, …) also surface here.
                match rubix_flows::convert("lint://input", yaml) {
                    Ok((_id, _rev, body)) => {
                        // Constants duplicated from
                        // `starter_flow_nodes::tool_call`
                        // (KIND_ID, TOOL_INPUT_SLOT) — that crate
                        // is a dev-dep of rubix-tools, not a
                        // regular dep, so we cannot import the
                        // symbols here. Hoist into a shared
                        // `starter-flow-spi`-side constants module
                        // when more lints land.
                        const TOOL_CALL_KIND_ID: &str = "starter.flow.tool-call";
                        const TOOL_INPUT_SLOT: &str = "input";
                        for n in &body.nodes {
                            if n.kind.as_str() != TOOL_CALL_KIND_ID {
                                continue;
                            }
                            let has_default = n
                                .settings
                                .get("tool_input")
                                .is_some_and(|v| !v.is_null());
                            let target =
                                format!("{}.{}", n.id, TOOL_INPUT_SLOT);
                            let has_link =
                                body.links.iter().any(|l| l.to == target);
                            if has_default || has_link {
                                continue;
                            }
                            errors.push(LintDiagnostic {
                                message: format!(
                                    "tool-call node `{}` has no \
                                     `settings.tool_input` and no inbound \
                                     link to `{}.{}` — the verb would be \
                                     invoked with no payload. Add a YAML \
                                     default or wire an upstream link.",
                                    n.id, n.id, TOOL_INPUT_SLOT,
                                ),
                                line: None,
                                column: None,
                            });
                        }
                    }
                    Err(e) => errors.push(load_error_to_diag(&e)),
                }
            }
            Err(e) => errors.push(load_error_to_diag(&e)),
        }

        let summary = if errors.is_empty() {
            Diagnostic::new(
                MessageKey::parse("rubix.flow.linted").expect("hard-coded key parses"),
            )
        } else {
            Diagnostic::new(
                MessageKey::parse("rubix.flow.lint.found_errors")
                    .expect("hard-coded key parses"),
            )
            .with_param("count", DiagnosticParam::I64(errors.len() as i64))
        };

        let response = FlowLintResponse { summary, errors };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

/// Convert a `rubix_flows::LoadError` into a [`LintDiagnostic`].
/// `serde_yaml::Error` carries a `location()` we surface when
/// present; other variants render without a position.
fn load_error_to_diag(err: &rubix_flows::LoadError) -> LintDiagnostic {
    let message = format!("{err}");
    let (line, column) = match err {
        rubix_flows::LoadError::Yaml { source, .. } => match source.location() {
            Some(loc) => (Some(loc.line() as u32), Some(loc.column() as u32)),
            None => (None, None),
        },
        _ => (None, None),
    };
    LintDiagnostic {
        message,
        line,
        column,
    }
}

/// Reject any `starter.flow.tool-call` node whose `tool_input` slot
/// has neither a YAML default (`settings.tool_input`) nor an
/// inbound link writing to `<node>.input`. The check lives inline
/// in [`FlowLintTool::invoke`] above so it can name the `FlowBody`
/// type without importing `starter-flow` (which is a dev-dep, not a
/// regular dep). This is the structural bug that bit stage 02 of
/// the data-flow scenario: the tool-call body forwards its `input`
/// slot to the tool verbatim, and the host-side seed adapter only
/// projects a value when YAML supplies one. Without either source
/// the slot is permanently stale. See
/// `rubix/docs/sessions/data-flow/02-ingest-l1-blockers-2026-05-26.md`.
const _LINT_DOC_ANCHOR: () = ();

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_YAML: &str = "id: com.x.a\ntrigger: explicit\nnodes:\n  - id: agent\n    kind: ai-agent\n    config: {}\nlinks: []\n";

    #[tokio::test]
    async fn clean_body_lints_ok() {
        let tool = FlowLintTool::new();
        let out = tool
            .invoke(serde_json::json!({"body_yaml": GOOD_YAML}))
            .await
            .unwrap();
        let resp: FlowLintResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.flow.linted");
        assert!(resp.errors.is_empty());
    }

    #[tokio::test]
    async fn broken_yaml_yields_found_errors_with_line_number() {
        let tool = FlowLintTool::new();
        // Tab where a space should be → serde_yaml errors with a
        // line/column location.
        let bad = "id: com.x.a\n\tnope: 1\n";
        let out = tool
            .invoke(serde_json::json!({"body_yaml": bad}))
            .await
            .unwrap();
        let resp: FlowLintResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.flow.lint.found_errors");
        assert_eq!(resp.errors.len(), 1);
        assert!(resp.errors[0].line.is_some(), "yaml errors should carry a line");
    }

    #[tokio::test]
    async fn empty_nodes_yields_found_errors() {
        let tool = FlowLintTool::new();
        let body = "id: com.x.a\ntrigger: explicit\nnodes: []\nlinks: []\n";
        let out = tool
            .invoke(serde_json::json!({"body_yaml": body}))
            .await
            .unwrap();
        let resp: FlowLintResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.flow.lint.found_errors");
    }

    #[tokio::test]
    async fn tool_call_with_neither_default_nor_link_is_flagged() {
        // Reproduces the bug from stage 02: a tool-call node with
        // no `tool_input` default and no inbound `.input` link is
        // structurally broken — the verb is invoked with an empty
        // payload every fire.
        let bad = r#"id: com.x.broken
trigger: explicit
nodes:
  - id: tick
    kind: starter.flow.trigger.schedule
    config:
      cron_expr: "*/5 * * * * *"
  - id: ingest
    kind: starter.flow.tool-call
    config:
      tool_id: "rubix.warehouse.ingest"
links:
  - { from: "tick.fire", to: "ingest.in" }
"#;
        let tool = FlowLintTool::new();
        let out = tool.invoke(serde_json::json!({"body_yaml": bad})).await.unwrap();
        let resp: FlowLintResponse = serde_json::from_value(out).unwrap();
        assert_eq!(
            resp.summary.code.as_str(),
            "rubix.flow.lint.found_errors",
            "resp: {resp:?}",
        );
        assert!(
            resp.errors.iter().any(|d| d.message.contains("ingest")
                && d.message.contains("tool_input")),
            "expected a tool_input lint for `ingest`; got {:?}",
            resp.errors,
        );
    }

    #[tokio::test]
    async fn tool_call_with_inbound_input_link_is_accepted() {
        // The producer flow's shape after the B2 fix: ingest has
        // an inbound link to its `input` slot, so no default is
        // required.
        let good = r#"id: com.x.ok
trigger: explicit
nodes:
  - id: synth
    kind: starter.flow.tool-call
    config:
      tool_id: "rubix.dataflow.synth.emit"
      tool_input:
        tenant_id: "site-a"
  - id: ingest
    kind: starter.flow.tool-call
    config:
      tool_id: "rubix.warehouse.ingest"
links:
  - { from: "synth.output", to: "ingest.input" }
  - { from: "synth.output", to: "ingest.in" }
"#;
        let tool = FlowLintTool::new();
        let out = tool.invoke(serde_json::json!({"body_yaml": good})).await.unwrap();
        let resp: FlowLintResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.flow.linted", "resp: {resp:?}");
    }
}
