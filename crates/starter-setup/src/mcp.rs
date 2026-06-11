//! MCP tool surface (P4, DOCS §11). Four tools sharing the same
//! [`RunService`] the REST surface uses — the adapters are thin.
//!
//! - `setup.list_templates` → templates the principal may run.
//! - `setup.run_template { template_id, input }` → `{ run_id }`.
//! - `setup.run_status { run_id }` → progress snapshot (MCP has no SSE).
//! - `setup.resume_run { run_id }`.
//!
//! Identity is host-bound: each tool reads the verified `Principal` from
//! the MCP transport's task-local (`starter_mcp::current_principal`),
//! never from the tool input — mirroring the REST `Extension<Principal>`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use starter_flow_spi::flow::RunId;
use starter_spi::error::{Error as SpiError, Result as SpiResult};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::authz::team_check;
use crate::service::RunService;
use starter_setup_spi::error::SetupError;
use starter_setup_spi::model::TemplateId;
use starter_setup_spi::store::{SetupRunStore, TemplateFilter, TemplateStore};

fn map_err(e: SetupError) -> SpiError {
    match e {
        SetupError::NotFound(what) => SpiError::NotFound { what },
        SetupError::Forbidden(_) => SpiError::Forbidden,
        SetupError::InvalidInput(message)
        | SetupError::InvalidYaml(message)
        | SetupError::InvalidBody(message)
        | SetupError::InvalidBinding(message)
        | SetupError::InvalidVersion(message)
        | SetupError::InvalidRunState(message) => SpiError::Invalid { message },
        other => SpiError::Internal {
            source: Box::new(other),
        },
    }
}

fn principal() -> SpiResult<starter_spi::auth::Principal> {
    starter_mcp::current_principal().ok_or(SpiError::Unauthenticated)
}

fn parse_run_id(input: &serde_json::Value) -> SpiResult<RunId> {
    let s = input
        .get("run_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SpiError::Invalid {
            message: "missing `run_id`".into(),
        })?;
    s.parse::<uuid::Uuid>().map(RunId).map_err(|_| SpiError::Invalid {
        message: "bad `run_id`".into(),
    })
}

/// `setup.list_templates`.
pub struct ListTemplatesTool<TS, RS> {
    service: Arc<RunService<TS, RS>>,
}

/// `setup.run_template`.
pub struct RunTemplateTool<TS, RS> {
    service: Arc<RunService<TS, RS>>,
}

/// `setup.run_status`.
pub struct RunStatusTool<TS, RS> {
    service: Arc<RunService<TS, RS>>,
}

/// `setup.resume_run`.
pub struct ResumeRunTool<TS, RS> {
    service: Arc<RunService<TS, RS>>,
}

/// Construct all four setup MCP tools over a shared service. Register
/// each into the host's `ToolRegistry`.
pub fn tools<TS, RS>(
    service: Arc<RunService<TS, RS>>,
) -> (
    ListTemplatesTool<TS, RS>,
    RunTemplateTool<TS, RS>,
    RunStatusTool<TS, RS>,
    ResumeRunTool<TS, RS>,
)
where
    TS: TemplateStore,
    RS: SetupRunStore,
{
    (
        ListTemplatesTool {
            service: service.clone(),
        },
        RunTemplateTool {
            service: service.clone(),
        },
        RunStatusTool {
            service: service.clone(),
        },
        ResumeRunTool { service },
    )
}

#[async_trait]
impl<TS, RS> Tool for ListTemplatesTool<TS, RS>
where
    TS: TemplateStore,
    RS: SetupRunStore,
{
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "setup.list_templates".into(),
            description: "List setup automation templates the caller may run.".into(),
            input_schema: json!({ "type": "object", "additionalProperties": false }),
        }
    }
    async fn invoke(&self, _input: serde_json::Value) -> SpiResult<serde_json::Value> {
        let p = principal()?;
        let list = self
            .service
            .templates()
            .list(TemplateFilter {
                tenant_id: p.tenant_id.clone(),
                category: None,
            })
            .await
            .map_err(map_err)?;
        Ok(json!({ "templates": list }))
    }
}

#[async_trait]
impl<TS, RS> Tool for RunTemplateTool<TS, RS>
where
    TS: TemplateStore,
    RS: SetupRunStore,
{
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "setup.run_template".into(),
            description: "Launch a setup automation template; returns its run id immediately."
                .into(),
            input_schema: json!({
                "type": "object",
                "required": ["template_id", "input"],
                "additionalProperties": false,
                "properties": {
                    "template_id": { "type": "string" },
                    "input": { "type": "object" }
                }
            }),
        }
    }
    async fn invoke(&self, input: serde_json::Value) -> SpiResult<serde_json::Value> {
        let p = principal()?;
        let template_id = input
            .get("template_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SpiError::Invalid {
                message: "missing `template_id`".into(),
            })?;
        let form = input.get("input").cloned().unwrap_or_else(|| json!({}));
        let template = self
            .service
            .templates()
            .get(p.tenant_id.as_deref(), &TemplateId(template_id.to_string()), None)
            .await
            .map_err(map_err)?
            .ok_or_else(|| SpiError::NotFound {
                what: template_id.to_string(),
            })?;
        // DOCS §10 step 2 — setup-layer team check.
        team_check(&template, &p).map_err(map_err)?;
        let handle = self
            .service
            .run_template(&template, &p, &form)
            .await
            .map_err(map_err)?;
        Ok(json!({ "run_id": handle.run }))
    }
}

#[async_trait]
impl<TS, RS> Tool for RunStatusTool<TS, RS>
where
    TS: TemplateStore,
    RS: SetupRunStore,
{
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "setup.run_status".into(),
            description: "Fetch a setup run's progress snapshot (poll; MCP has no streaming)."
                .into(),
            input_schema: json!({
                "type": "object",
                "required": ["run_id"],
                "additionalProperties": false,
                "properties": { "run_id": { "type": "string" } }
            }),
        }
    }
    async fn invoke(&self, input: serde_json::Value) -> SpiResult<serde_json::Value> {
        let p = principal()?;
        let run_id = parse_run_id(&input)?;
        let run = self
            .service
            .runs()
            .get(run_id)
            .await
            .map_err(map_err)?
            .ok_or_else(|| SpiError::NotFound {
                what: run_id.to_string(),
            })?;
        if run.owner != p.subject && !p.is_super_admin() {
            return Err(SpiError::Forbidden);
        }
        Ok(serde_json::to_value(&run).unwrap_or_else(|_| json!({})))
    }
}

#[async_trait]
impl<TS, RS> Tool for ResumeRunTool<TS, RS>
where
    TS: TemplateStore,
    RS: SetupRunStore,
{
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "setup.resume_run".into(),
            description: "Resume a failed, resumable setup run from its cursor.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["run_id"],
                "additionalProperties": false,
                "properties": { "run_id": { "type": "string" } }
            }),
        }
    }
    async fn invoke(&self, input: serde_json::Value) -> SpiResult<serde_json::Value> {
        let p = principal()?;
        let run_id = parse_run_id(&input)?;
        let run = self
            .service
            .runs()
            .get(run_id)
            .await
            .map_err(map_err)?
            .ok_or_else(|| SpiError::NotFound {
                what: run_id.to_string(),
            })?;
        if run.owner != p.subject && !p.is_super_admin() {
            return Err(SpiError::Forbidden);
        }
        let template = self
            .service
            .templates()
            .get(run.tenant_id.as_deref(), &run.template_id, Some(run.template_version))
            .await
            .map_err(map_err)?
            .ok_or_else(|| SpiError::NotFound {
                what: run.template_id.0.clone(),
            })?;
        let handle = self
            .service
            .resume_run(&template, run_id)
            .await
            .map_err(map_err)?;
        Ok(json!({ "run_id": handle.run }))
    }
}
