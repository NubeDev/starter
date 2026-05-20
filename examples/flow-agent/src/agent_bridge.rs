//! Agent-as-tool bridge — the load-bearing half of stage 5.
//!
//! Sits between [`crate::ai_runtime::AiRuntime`] and the flow engine.
//! Adds three concerns on `AiRuntime` via an `impl` block split
//! across files: `synthesize_flow_tools` (build `ToolDef`s from the
//! flow registry), `drive_chat` (the multi-turn loop), and
//! `invoke_flow_tool` / `dispatch_tool_use` (fire a flow as a tool
//! call and surface its terminal output to the model on the next
//! turn). Free-function helpers (`drain_run`, slot → JSON
//! converters) live in [`crate::run_drain`].

use std::sync::Arc;

use serde_json::{json, Value as JsonValue};
use tokio::sync::mpsc;

use starter_ai::TokenCancel;
use starter_spi::ai::{
    AiRunner, CliCfg, Event, EventKind, HistoryMessage, Provider, RestCfg, RunnerInput, SessionId,
    ToolChoice, ToolDef, ToolUse,
};

use crate::ai_runtime::{AiRuntime, MAX_AGENT_TURNS};
use crate::domain::{Agent, FlowSummary};
use crate::flow_engine::FlowEngineError;
use crate::run_drain::drain_run;
use crate::sse::RunEvent;

impl AiRuntime {
    /// Synthesise an [`AiTool`]/`ToolDef` per flow the agent is
    /// allowed to call. The agent's `tools` array names flows by id;
    /// the wildcard `flow:*` lights every flow in the registry.
    /// Returns an empty vec when the agent declares no flow tools.
    pub async fn synthesize_flow_tools(
        &self,
        agent_tools: &[String],
    ) -> Result<Vec<ToolDef>, crate::ai_runtime::AgentRunError> {
        use crate::ai_runtime::AgentRunError;
        if agent_tools.is_empty() {
            return Ok(Vec::new());
        }
        let wildcard = agent_tools.iter().any(|t| t == "flow:*");
        let specific: Vec<&str> = agent_tools
            .iter()
            .filter_map(|t| t.strip_prefix("flow:"))
            .filter(|s| !s.is_empty() && *s != "*")
            .collect();
        if !wildcard && specific.is_empty() {
            return Ok(Vec::new());
        }

        let flows = self
            .flows()
            .list()
            .await
            .map_err(|e| AgentRunError::Registry(e.to_string()))?;

        let mut out = Vec::new();
        for FlowSummary {
            id,
            name,
            description,
            ..
        } in flows
        {
            let included = wildcard || specific.iter().any(|s| *s == id || *s == name);
            if !included {
                continue;
            }
            let desc = description.unwrap_or_else(|| name.clone());
            out.push(ToolDef {
                name: format!("flow:{}", id),
                description: Some(desc),
                input_schema: permissive_object_schema(),
            });
        }
        Ok(out)
    }

    /// Fire a flow as if it were a tool call. Persists a run row,
    /// emits run-events on the shared `EventHub.runs` channel, and
    /// returns the terminal output map as a JSON string (or a
    /// `{"error": …}` payload if the engine rejected the fire / the
    /// run failed).
    pub async fn invoke_flow_tool(&self, flow_id: &str, payload: JsonValue) -> String {
        let flow = match self.flows().get(flow_id).await {
            Ok(f) => f,
            Err(e) => return error_string(format!("flow `{flow_id}` not found: {e}")),
        };

        let outcome = match self.engine().fire(&flow.id, &flow.graph, payload).await {
            Ok(o) => o,
            Err(FlowEngineError::Parse(e)) => {
                return error_string(format!("flow `{flow_id}` graph parse error: {e}"));
            }
            Err(FlowEngineError::Invalid(msg)) => {
                return error_string(format!("flow `{flow_id}` invalid: {msg}"));
            }
            Err(FlowEngineError::Engine(msg)) => {
                return error_string(format!("flow `{flow_id}` engine error: {msg}"));
            }
            Err(FlowEngineError::Fire(msg)) => {
                return error_string(format!("flow `{flow_id}` fire failed: {msg}"));
            }
        };

        let run = match self.runs_store().record_started(&flow.id).await {
            Ok(r) => r,
            Err(e) => return error_string(format!("record run start: {e}")),
        };
        let _ = self.hub().runs.send(RunEvent::RunStarted {
            flow_id: flow.id.clone(),
            run_id: run.id.clone(),
        });

        let (status, trace, output) =
            drain_run(self.hub().clone(), flow.id.clone(), run.id.clone(), outcome).await;
        if let Err(e) = self
            .runs_store()
            .record_finished(&run.id, &status, trace.as_ref())
            .await
        {
            tracing::error!(error = %e, run_id = %run.id, "record run finished");
        }
        let _ = self.hub().runs.send(RunEvent::RunFinished {
            flow_id: flow.id.clone(),
            run_id: run.id.clone(),
            status: status.clone(),
        });

        match status.as_str() {
            "ok" => serde_json::to_string(&output).unwrap_or_else(|_| "{}".to_owned()),
            other => error_string(format!(
                "flow run finished with status `{other}`: {}",
                trace
                    .as_ref()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "no trace".to_owned())
            )),
        }
    }

    /// Chat-stream loop driver. Each turn hands the runner the
    /// synthesised flow `ToolDef`s; tool calls that match a `flow:*`
    /// name are dispatched through [`FlowEngine`] and their terminal
    /// output is fed back as a `user`-role history message before the
    /// next turn. CLI-shape runners (Claude CLI) manage their own
    /// tool loop, so the `cli_path` short-circuits to a single-shot.
    pub(crate) async fn drive_chat(
        &self,
        provider: Provider,
        runner: Arc<dyn AiRunner>,
        agent: Agent,
        prompt: String,
        mut history: Vec<HistoryMessage>,
        sse_tx: mpsc::Sender<String>,
    ) -> Result<(), String> {
        let flow_tools = self
            .synthesize_flow_tools(&agent.tools)
            .await
            .map_err(|e| e.to_string())?;
        let cli_path =
            matches!(provider, Provider::Claude | Provider::Codex | Provider::Copilot);
        if cli_path || flow_tools.is_empty() {
            return drive_single_shot(provider, runner, agent, prompt, history, sse_tx).await;
        }

        history.push(HistoryMessage {
            role: "user".into(),
            content: prompt.clone(),
        });

        for turn in 0..MAX_AGENT_TURNS {
            let (ev_tx, mut ev_rx) = mpsc::channel::<Event>(32);
            let pump_tx = sse_tx.clone();
            let pump = tokio::spawn(async move {
                while let Some(ev) = ev_rx.recv().await {
                    if let Some(payload) = event_to_payload(&ev) {
                        if pump_tx.send(payload).await.is_err() {
                            break;
                        }
                    }
                }
            });

            let cfg = RestCfg {
                prompt: prompt.clone(),
                system_prompt: agent.system_prompt.clone(),
                model: Some(agent.model.clone()),
                history: history.clone(),
                tools: flow_tools.clone(),
                tool_choice: Some(ToolChoice::Auto),
                ..RestCfg::default()
            };
            let session_id = SessionId::from(format!("agent-{}-turn-{turn}", agent.id));
            let cancel = TokenCancel::new();
            let run_res = runner
                .run(RunnerInput::Rest(cfg), session_id, ev_tx, &cancel)
                .await;
            let _ = pump.await;

            let result = match run_res {
                Ok(r) => r,
                Err(e) => return Err(format!("runner failed: {e}")),
            };
            if let Some(upstream) = result.error.clone() {
                return Err(format!("upstream error: {upstream}"));
            }
            if !result.text.is_empty() {
                history.push(HistoryMessage {
                    role: "assistant".into(),
                    content: result.text.clone(),
                });
            }
            if result.tool_uses.is_empty() {
                return Ok(());
            }
            for tu in &result.tool_uses {
                let reply = self.dispatch_tool_use(tu, &sse_tx).await;
                history.push(HistoryMessage {
                    role: "user".into(),
                    content: reply,
                });
            }
        }
        Err(format!(
            "agent loop exceeded MAX_AGENT_TURNS={MAX_AGENT_TURNS}"
        ))
    }

    /// Dispatch one `ToolUse` from the model. Only `flow:*` names are
    /// handled; everything else returns a refusal string so the model
    /// can re-plan. Surfaces a `tool-call` + `tool-result` SSE frame
    /// pair on the supplied chat sender.
    pub(crate) async fn dispatch_tool_use(
        &self,
        tu: &ToolUse,
        sse_tx: &mpsc::Sender<String>,
    ) -> String {
        let flow_id = match tu.name.strip_prefix("flow:") {
            Some(rest) if !rest.is_empty() => rest.to_string(),
            _ => {
                return format!(
                    "tool `{}` refused: only `flow:<id>` tools are dispatched by the host runtime",
                    tu.name
                );
            }
        };
        let start_payload = json!({
            "type": "tool-call",
            "toolCall": {
                "id": tu.id,
                "name": tu.name,
                "args": tu.input,
                "state": "running",
            },
        })
        .to_string();
        let _ = sse_tx.send(start_payload).await;

        let reply = self.invoke_flow_tool(&flow_id, tu.input.clone()).await;

        let done_payload = json!({
            "type": "tool-result",
            "toolCall": {
                "id": tu.id,
                "name": tu.name,
                "result": reply,
                "state": "done",
            },
        })
        .to_string();
        let _ = sse_tx.send(done_payload).await;

        format!("tool `{}` (id={}) returned: {}", tu.name, tu.id, reply)
    }
}

/// Single-turn driver for paths that don't dispatch flow tools
/// (CLI runners, or REST agents with no `flow:*` tools declared).
async fn drive_single_shot(
    provider: Provider,
    runner: Arc<dyn AiRunner>,
    agent: Agent,
    prompt: String,
    history: Vec<HistoryMessage>,
    sse_tx: mpsc::Sender<String>,
) -> Result<(), String> {
    let input = build_input(&provider, &agent, prompt, history);
    let session_id = SessionId::from(format!("agent-{}", agent.id));

    let (ev_tx, mut ev_rx) = mpsc::channel::<Event>(32);
    let pump_tx = sse_tx.clone();
    let pump = tokio::spawn(async move {
        while let Some(ev) = ev_rx.recv().await {
            if let Some(payload) = event_to_payload(&ev) {
                if pump_tx.send(payload).await.is_err() {
                    break;
                }
            }
        }
    });

    let cancel = TokenCancel::new();
    let run_res = runner.run(input, session_id, ev_tx, &cancel).await;
    let _ = pump.await;

    match run_res {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("runner failed: {e}")),
    }
}

/// Build the runner input variant the resolved provider expects.
fn build_input(
    provider: &Provider,
    agent: &Agent,
    prompt: String,
    history: Vec<HistoryMessage>,
) -> RunnerInput {
    match provider {
        Provider::Claude | Provider::Codex | Provider::Copilot => {
            // CLI runners take a single prompt + optional system
            // context. Fold history into the system prompt so the
            // model still sees prior turns.
            let folded_history = fold_history_for_cli(&history);
            let system_prompt = match (agent.system_prompt.as_deref(), folded_history.as_str()) {
                (None, "") => None,
                (Some(s), "") => Some(s.to_owned()),
                (None, h) => Some(h.to_owned()),
                (Some(s), h) => Some(format!("{s}\n\n# Prior conversation\n{h}")),
            };
            RunnerInput::Cli(CliCfg {
                prompt,
                system_prompt,
                model: Some(agent.model.clone()),
                permission_mode: Some(starter_spi::ai::PermissionMode::Bypass),
                ..CliCfg::default()
            })
        }
        Provider::Anthropic | Provider::OpenAi => RunnerInput::Rest(RestCfg {
            prompt,
            system_prompt: agent.system_prompt.clone(),
            model: Some(agent.model.clone()),
            history,
            ..RestCfg::default()
        }),
    }
}

fn fold_history_for_cli(history: &[HistoryMessage]) -> String {
    history
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert an `Event` from the runner to the SSE `data:` payload
/// shape the chat adapter expects. Returns `None` for events the chat
/// surface doesn't render (e.g. `Connected`, `Done` — the latter is
/// replaced with the literal `[DONE]` sentinel by the caller).
fn event_to_payload(ev: &Event) -> Option<String> {
    let payload = match &ev.kind {
        EventKind::Text { content } => json!({ "type": "text", "text": content }),
        EventKind::ToolUse { id, name, input } => json!({
            "type": "tool-call",
            "toolCall": {
                "id": id.clone().unwrap_or_default(),
                "name": name,
                "args": input.clone().unwrap_or(serde_json::Value::Null),
                "state": "running",
            },
        }),
        EventKind::Error { message } => json!({ "type": "error", "error": message }),
        EventKind::Connected { .. } | EventKind::Done { .. } => return None,
    };
    Some(payload.to_string())
}

fn permissive_object_schema() -> JsonValue {
    json!({
        "type": "object",
        "additionalProperties": true,
    })
}

fn error_string(msg: String) -> String {
    json!({ "error": msg }).to_string()
}
