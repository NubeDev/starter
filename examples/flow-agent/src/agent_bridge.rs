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

    /// Synthesise `insights:*` tools from the agent's `tools` array.
    /// Returns the 5 Phase-2 rule tools when the agent declares any
    /// `insights:rule.*` entry (or the wildcards `insights:*` /
    /// `insights:rule.*`), the 2 Phase-3 verdict tools for
    /// `insights:verdict.*`, and the 3 Phase-4 pipeline tools for
    /// `insights:pipeline.*`. No-op when `with_insights` wasn't
    /// called on the runtime.
    pub fn synthesize_insights_tools(&self, agent_tools: &[String]) -> Vec<ToolDef> {
        if self.insights().is_none() || agent_tools.is_empty() {
            return Vec::new();
        }
        let want = |prefix: &str| {
            agent_tools
                .iter()
                .any(|t| t == "insights:*" || t == prefix || t.starts_with(prefix))
        };
        let mut out = Vec::new();
        if want("insights:rule.") {
            out.extend(rule_tool_defs(agent_tools));
        }
        if want("insights:verdict.") {
            out.extend(verdict_tool_defs(agent_tools));
        }
        if want("insights:pipeline.") {
            out.extend(pipeline_tool_defs(agent_tools));
        }
        out
    }

    /// Dispatch one `insights:*` tool call against fixtures.
    /// Returns the stringified JSON the agent sees in its next turn.
    pub async fn dispatch_insights_tool(&self, tu: &ToolUse) -> String {
        let Some(state) = self.insights() else {
            return error_string("insights tools unavailable: runtime not bound".into());
        };
        let input = tu.input.clone();
        match tu.name.as_str() {
            "insights:rule.list" => {
                let g = state.data.read().await;
                JsonValue::Array(g.rules.clone()).to_string()
            }
            "insights:rule.read" => {
                let id = input.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let g = state.data.read().await;
                g.rules
                    .iter()
                    .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| error_string(format!("rule `{id}` not found")))
            }
            "insights:rule.propose" => {
                // Per spec §Agent tools, propose returns a *proposal*,
                // not a write. The operator clicks Approve in the UI;
                // the agent then calls `rule.apply`.
                json!({
                    "type": "proposal",
                    "action": "create-or-update",
                    "rule": input,
                    "needs_approval": true,
                })
                .to_string()
            }
            "insights:rule.apply" => {
                let body = input.clone();
                let id = match body.get("id").and_then(|v| v.as_str()) {
                    Some(s) if !s.is_empty() => s.to_string(),
                    _ => return error_string("rule.apply: missing `id` field".into()),
                };
                let mut g = state.data.write().await;
                if let Some(existing) = g
                    .rules
                    .iter_mut()
                    .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
                {
                    *existing = body.clone();
                } else {
                    g.rules.push(body.clone());
                }
                let snapshot = g.rules.clone();
                if let Err(e) = g.persist_array("rules.json", &snapshot) {
                    return error_string(format!("rule.apply: persist failed: {e}"));
                }
                json!({ "ok": true, "id": id }).to_string()
            }
            "insights:rule.dry-run" => {
                let id = input.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let g = state.data.read().await;
                if !g
                    .rules
                    .iter()
                    .any(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
                {
                    return error_string(format!("rule `{id}` not found"));
                }
                let latest = g
                    .verdicts
                    .iter()
                    .filter(|v| v.get("rule_id").and_then(|x| x.as_str()) == Some(id))
                    .max_by_key(|v| {
                        v.get("at")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string()
                    });
                match latest {
                    Some(v) => {
                        let mut clone = v.clone();
                        if let Some(obj) = clone.as_object_mut() {
                            obj.insert("dry_run".into(), JsonValue::Bool(true));
                        }
                        clone.to_string()
                    }
                    None => json!({
                        "id": format!("dry-{id}"),
                        "rule_id": id,
                        "dry_run": true,
                        "severity": "Healthy",
                        "summary": "no historical verdicts for this rule",
                    })
                    .to_string(),
                }
            }
            "insights:verdict.query" => {
                let g = state.data.read().await;
                let rule_id = input.get("rule_id").and_then(|v| v.as_str());
                let sev = input.get("severity").and_then(|v| v.as_str());
                let tag = input.get("tag").and_then(|v| v.as_str());
                let out: Vec<JsonValue> = g
                    .verdicts
                    .iter()
                    .filter(|v| {
                        if let Some(r) = rule_id {
                            if v.get("rule_id").and_then(|x| x.as_str()) != Some(r) {
                                return false;
                            }
                        }
                        if let Some(s) = sev {
                            if v.get("severity").and_then(|x| x.as_str()) != Some(s) {
                                return false;
                            }
                        }
                        if let Some(t) = tag {
                            let tags = v.get("tags").and_then(|x| x.as_array());
                            if !tags.is_some_and(|arr| arr.iter().any(|x| x.as_str() == Some(t))) {
                                return false;
                            }
                        }
                        true
                    })
                    .cloned()
                    .collect();
                JsonValue::Array(out).to_string()
            }
            "insights:verdict.explain" => {
                let id = input.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let g = state.data.read().await;
                let v = g
                    .verdicts
                    .iter()
                    .find(|v| v.get("id").and_then(|x| x.as_str()) == Some(id));
                match v {
                    Some(v) => v.to_string(),
                    None => error_string(format!("verdict `{id}` not found")),
                }
            }
            "insights:pipeline.read" => {
                let id = input.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let g = state.data.read().await;
                g.pipelines
                    .iter()
                    .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(id))
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| error_string(format!("pipeline `{id}` not found")))
            }
            "insights:pipeline.propose-edit" => json!({
                "type": "proposal",
                "action": "pipeline-edit",
                "patch": input,
                "needs_approval": true,
            })
            .to_string(),
            "insights:pipeline.apply-edit" => {
                let body = input.clone();
                let id = match body.get("id").and_then(|v| v.as_str()) {
                    Some(s) if !s.is_empty() => s.to_string(),
                    _ => {
                        return error_string("pipeline.apply-edit: missing `id` field".into());
                    }
                };
                let mut g = state.data.write().await;
                if let Some(existing) = g
                    .pipelines
                    .iter_mut()
                    .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
                {
                    *existing = body.clone();
                } else {
                    g.pipelines.push(body.clone());
                }
                let snapshot = g.pipelines.clone();
                if let Err(e) = g.persist_array("pipelines.json", &snapshot) {
                    return error_string(format!("pipeline.apply-edit: persist failed: {e}"));
                }
                json!({ "ok": true, "id": id }).to_string()
            }
            other => error_string(format!("unknown insights tool: `{other}`")),
        }
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
        let mut tools = self
            .synthesize_flow_tools(&agent.tools)
            .await
            .map_err(|e| e.to_string())?;
        tools.extend(self.synthesize_insights_tools(&agent.tools));
        let flow_tools = tools;
        let cli_path = matches!(
            provider,
            Provider::Claude | Provider::Codex | Provider::Copilot
        );
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
        // Insights tools — fixture-backed dispatch, no engine.
        if tu.name.starts_with("insights:") {
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
            let reply = self.dispatch_insights_tool(tu).await;
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
            return format!("tool `{}` (id={}) returned: {}", tu.name, tu.id, reply);
        }

        let flow_id = match tu.name.strip_prefix("flow:") {
            Some(rest) if !rest.is_empty() => rest.to_string(),
            _ => {
                return format!(
                    "tool `{}` refused: only `flow:<id>` and `insights:<name>` tools are dispatched by the host runtime",
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

/// `insights:rule.*` tool defs. Filter by the agent's tools array so
/// an agent can opt into a subset (e.g. read-only).
fn rule_tool_defs(agent_tools: &[String]) -> Vec<ToolDef> {
    insights_subset(
        agent_tools,
        "insights:rule.",
        &[
            ("insights:rule.list", "List rules with their schema and tags."),
            ("insights:rule.read", "Read a single rule by id."),
            (
                "insights:rule.propose",
                "Propose a new rule or rule edit. Returns a proposal; operator must approve before apply.",
            ),
            (
                "insights:rule.apply",
                "Apply an approved proposal. Writes the rule to the fixture store.",
            ),
            (
                "insights:rule.dry-run",
                "Synthesise a verdict for the rule from fixture data (no engine).",
            ),
        ],
    )
}

fn verdict_tool_defs(agent_tools: &[String]) -> Vec<ToolDef> {
    insights_subset(
        agent_tools,
        "insights:verdict.",
        &[
            (
                "insights:verdict.query",
                "Filter verdicts by rule_id, tag, severity, since/until.",
            ),
            (
                "insights:verdict.explain",
                "Return a verdict row for narration; the agent supplies the prose.",
            ),
        ],
    )
}

fn pipeline_tool_defs(agent_tools: &[String]) -> Vec<ToolDef> {
    insights_subset(
        agent_tools,
        "insights:pipeline.",
        &[
            ("insights:pipeline.read", "Read a pipeline graph by id."),
            (
                "insights:pipeline.propose-edit",
                "Propose a pipeline graph edit. Returns a proposal; needs operator approval.",
            ),
            (
                "insights:pipeline.apply-edit",
                "Apply an approved pipeline edit.",
            ),
        ],
    )
}

fn insights_subset(agent_tools: &[String], prefix: &str, defs: &[(&str, &str)]) -> Vec<ToolDef> {
    let wildcard_name = format!("{prefix}*"); // e.g. `insights:rule.*`
    let wildcard = agent_tools
        .iter()
        .any(|t| t == "insights:*" || t == prefix || *t == wildcard_name);
    defs.iter()
        .filter(|(name, _)| wildcard || agent_tools.iter().any(|t| t == name))
        .map(|(name, desc)| ToolDef {
            name: (*name).to_owned(),
            description: Some((*desc).to_owned()),
            input_schema: permissive_object_schema(),
        })
        .collect()
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
