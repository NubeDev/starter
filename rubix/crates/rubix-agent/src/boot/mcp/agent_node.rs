//! `com.rubix.ai-agent` [`NodeBehavior`] implementation.
//!
//! Thin adapter binding the kind id every bundled rubix YAML uses
//! (`com.rubix.ai-agent`) to [`starter_ai_agent::AgentLoop`]. The
//! seed adapter at [`super::register`] writes a JSON payload
//! containing `locale`, `prefs`, and the caller's MCP `arguments`
//! JSON onto the `payload` slot; this body builds a prompt from
//! that payload, drives the agent loop, and writes the reply to
//! the `out` slot the output adapter reads back.
//!
//! Behaviour is deliberately split into two paths so the
//! deterministic part of the response is independent of the
//! non-deterministic LLM round-trip:
//!
//! 1. **Primary-tool dispatch.** Each node with a mapped primary
//!    tool dispatches it; the structured `Diagnostic` IS the
//!    response. This is the smoke-test path.
//! 2. **LLM narration.** On by default; disable with
//!    `RUBIX_AI_NARRATION=0` for pure-tool responses (no LLM cost,
//!    deterministic CI). The agent loop today returns only a
//!    free-form reply (see crates/starter-ai-agent/LONG-TERM.md
//!    §"CLI runner tool dispatch"); failures do not fail the node
//!    so the deterministic tool output still reaches the caller.
//!    Long-running narration awaits no longer race the run
//!    coordinator's quiescence window — the in-flight node tracker
//!    in starter-flow's run coordinator holds completion until
//!    `NodeEmitted` arrives.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use starter_ai_agent::{AgentLoop, ToolSet};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};

use starter_spi::ai::{AiRunner, PermissionMode};
use starter_spi::tool::Tool;

/// `com.rubix.ai-agent` node kind.
pub(super) struct RubixAiAgentNode {
    kind: KindId,
    runner: Arc<dyn AiRunner>,
    tools: Vec<Arc<dyn Tool>>,
    /// Bearer-token-bridged MCP server URL the Claude CLI wrapper
    /// attaches to so the model can dispatch *host* tools mid-turn
    /// (orthogonal to the locally-dispatched [`Self::tools`] set
    /// the [`AgentLoop`] owns). `None` keeps the legacy catalogue-
    /// less behaviour, which is what every fixture test in this
    /// crate exercises. Snapshotted at construction from
    /// `RUBIX_SERVICE_MCP_URL` (see [`super::register`]); a follow-up
    /// will auto-derive this from the agent's own bind address so
    /// operators do not have to copy-paste a service token.
    mcp_url: Option<String>,
    /// Token paired with [`Self::mcp_url`]. Snapshotted at
    /// construction from `RUBIX_SERVICE_MCP_TOKEN`. Only meaningful
    /// when `mcp_url` is `Some`.
    mcp_token: Option<String>,
}

impl RubixAiAgentNode {
    pub(super) fn new(
        kind: KindId,
        runner: Arc<dyn AiRunner>,
        tools: Vec<Arc<dyn Tool>>,
        mcp_url: Option<String>,
        mcp_token: Option<String>,
    ) -> Self {
        Self {
            kind,
            runner,
            tools,
            mcp_url,
            mcp_token,
        }
    }

    fn find_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .iter()
            .find(|t| t.definition().name == name)
            .cloned()
    }
}

#[async_trait]
impl NodeBehavior for RubixAiAgentNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        // The seed adapter writes the locale + prefs + caller input
        // onto the `payload` slot as a JSON object. Read it back so
        // we can both (a) forward `input` to the primary tool's
        // `invoke` and (b) hand the caller context to the LLM as a
        // free-form prompt for the optional `reply` field.
        let payload = match input.get(rubix_flows::DEFAULT_SEED_SLOT) {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => json!({}),
        };
        let tool_input = payload
            .get("input")
            .cloned()
            .unwrap_or_else(|| json!({}));

        // Inject session-derived fields into the primary tool's
        // input when the caller omitted them. The MCP HTTP transport
        // binds the authenticated `Principal` on a task-local (see
        // `starter_mcp::current_principal`); without this top-up the
        // common case — a chat UI POST that only sends `{"prompt":
        // "..."}` — would fail every dashboard / user / tenant verb
        // with `missing field tenant_id`. Fields the caller did
        // supply win, so explicit MCP `arguments` keep overriding
        // the session defaults.
        let tool_input = augment_tool_input_with_principal(tool_input);

        // Primary-tool dispatch — the deterministic part of the
        // output. For nodes with a mapped primary tool, the tool's
        // output IS the structured response; the LLM reply is a
        // bonus narration field that the smoke test does not assert
        // against. For nodes without a mapping, fall back to a
        // reply-only response.
        //
        // The primary tool name is sourced from the per-invocation
        // seed payload (written by the per-flow seed adapter in
        // `super::register`). Keying off `ctx.node` is unsafe — every
        // rubix flow's root node uses the same id (`agent` / `check`),
        // so a NodeId-keyed lookup collides across flows.
        let primary_tool_name: Option<&str> = payload
            .get("primary_tool")
            .and_then(|v| v.as_str());
        // Primary-tool dispatch failures (unknown tool, bad input, etc.)
        // are NOT fatal: the node logs a warn and falls through to the
        // narration-only path so the caller still gets a response. The
        // common case driving this is an MCP `tools/call` with no
        // arguments against a flow whose first allowed_tool requires
        // fields (e.g. dashboard tools always need `tenant_id`). Failing
        // the whole flow run on that would produce a silent null body —
        // worse UX than a graceful "no tool output" narration.
        let tool_value: Option<Value> = match primary_tool_name {
            Some(tool_name) => match self.find_tool(tool_name) {
                None => {
                    tracing::warn!(
                        tool = %tool_name,
                        node = %ctx.node,
                        "ai-agent: primary tool not in registry; falling back to reply-only",
                    );
                    None
                }
                Some(tool) => match tool.invoke(tool_input).await {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!(
                            tool = %tool_name,
                            node = %ctx.node,
                            error = %e,
                            "ai-agent: primary tool failed; falling back to reply-only",
                        );
                        None
                    }
                },
            },
            None => None,
        };

        // LLM narration on by default. The starter-flow run
        // coordinator now tracks in-flight nodes so a long
        // `behavior.invoke` await no longer races the quiescence
        // window — see the `slow_node_body_does_not_race_quiescence`
        // test in starter-flow. Operators who want pure tool output
        // (no LLM cost / latency, deterministic CI) can disable with
        // `RUBIX_AI_NARRATION=0`.
        //
        // Failures in the agent loop never fail the node — the
        // deterministic tool output still reaches the caller.
        let locale = payload
            .get("locale")
            .and_then(|v| v.as_str())
            .unwrap_or("en");
        let narration_enabled = std::env::var("RUBIX_AI_NARRATION")
            .map(|v| !(v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off")))
            .unwrap_or(true);
        let reply = if narration_enabled {
            // Skill body (e.g. dashboard-builder playbook) lifted from
            // the seed payload, written by `boot::mcp::register::
            // skill_body_for_hint` when the flow YAML declared a
            // `skill_hint`. Prepending the body to the CLI prompt
            // primes the model with rubix's per-goal instructions;
            // the `skill_hint` field by itself is dropped at
            // `crates/starter-flow-nodes/src/ai_agent.rs:347` with a
            // warn until the starter-skills loader lands. Empty when
            // no skill is resolved; the surrounding prompt stays
            // intelligible either way.
            let skill_preamble = payload
                .get("skill_body")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| format!("# Skill instructions (follow these)\n\n{s}\n\n---\n\n"))
                .unwrap_or_default();
            let prompt = format!(
                "{skill_preamble}Summarise this rubix flow result in one sentence for the operator. \
                 Respond in BCP-47 locale `{locale}` (translate the summary into \
                 the matching language; e.g. `es-AR` → Spanish, `en-US` → English). \
                 Caller context (JSON): {payload}\n\nTool output (JSON): {}",
                tool_value
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "<none>".to_owned())
            );

            // Direct `AgentLoop` call carrying the MCP wiring so
            // the upstream Claude CLI also attaches to the rubix
            // `/api/v1/mcp` bridge and the model can dispatch host
            // tools mid-turn. The `with_mcp` builder is a no-op for
            // non-CLI runners (REST providers carry their tool list
            // through `RestCfg::tools`) and for tests using
            // `FixtureRunner` (both `mcp_url` and `mcp_token` stay
            // `None` in CI since the env vars are unset). The
            // local `ToolSet` is the in-process fallback the loop
            // dispatches when the recorded transcript / model
            // returns a `tool_uses` entry directly.
            // Restrict the wrapped Claude CLI to MCP-bridged tools
            // only. Without this, the binary's *built-in* tools are
            // all in scope, and the model reaches for the worst of
            // them (`AskUserQuestion`) instead of acting — turning
            // "make me an iot dashboard" into a multi-turn survey.
            // `mcp__rubix__*` matches every tool exposed through the
            // MCP server we generate in
            // `crates/starter-ai/src/runners/claude.rs` (server name
            // hard-coded to `"rubix"` there). The pattern is a no-op
            // for non-CLI runners and for `mcp_url == None` (no MCP
            // bridge in scope), so test fixtures stay unaffected.
            let allowed_pattern = self
                .mcp_url
                .as_ref()
                .map(|_| "mcp__rubix__*".to_owned());
            // Per-flow CLI built-in restriction, snapshotted into the
            // seed payload by `boot::mcp::register::register_one`.
            // `Some([])` (the rubix `tools: []` lockdown — stage 07
            // "MCP only, no built-ins") forwards an empty `--tools`
            // list to the wrapped Claude CLI so Bash / Read / Edit /
            // AskUserQuestion are all out of scope. Absent → `None`,
            // CLI default catalogue stays in scope.
            let cli_tools: Option<String> = payload
                .get("cli_tools")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                });
            // Structured stage-07 self-check log line: confirms per
            // run whether the skill loaded, whether the CLI built-in
            // surface was locked down, and which MCP allow pattern
            // the wrapper saw. One line is enough — when the next AI
            // session does something weird, this tells us which of
            // the three independent failure modes (skill body
            // missing, Bash leaked back in, MCP allow pattern drift)
            // is in play without re-running the flow.
            let skill_body_len: usize = payload
                .get("skill_body")
                .and_then(|v| v.as_str())
                .map(str::len)
                .unwrap_or(0);
            // Escape newlines/tabs so the field doesn't break the
            // log line across multiple physical lines — without this
            // a SKILL.md whose body starts within the first 80 chars
            // (e.g. stage 07's BANANA preamble) splits the structured
            // log into fragments that grep/journalctl filters miss.
            let skill_first_80: String = payload
                .get("skill_body")
                .and_then(|v| v.as_str())
                .map(|s| {
                    s.chars()
                        .take(80)
                        .collect::<String>()
                        .replace('\n', "\\n")
                        .replace('\t', "\\t")
                })
                .unwrap_or_default();
            tracing::info!(
                target: "rubix.ai_agent.self_check",
                node = %ctx.node,
                skill_hint = ?payload.get("primary_tool").and_then(|v| v.as_str()),
                skill_bytes_len = skill_body_len,
                skill_first_80 = %skill_first_80,
                cli_tools = ?cli_tools.as_deref(),
                mcp_allowed_pattern = ?allowed_pattern.as_deref(),
                "ai-agent run self-check"
            );
            let agent = AgentLoop::new(self.runner.clone(), ToolSet::new(self.tools.clone()))
                .with_mcp(self.mcp_url.clone(), self.mcp_token.clone())
                .with_allowed_tools(allowed_pattern)
                .with_cli_tools(cli_tools)
                // Bypass the CLI's interactive approval prompt. The
                // host has already gated the request at the HTTP
                // boundary (login + cookie); the model is acting on
                // a tool the operator implicitly approved by signing
                // in. Without this every `mcp__rubix__*` call hangs
                // on a stdin prompt the headless wrapper never
                // answers, which surfaces in the UI as "needs your
                // permission" no-op replies.
                .with_permission_mode(Some(PermissionMode::Bypass));
            // `run_with_outcome` always returns events collected up
            // to the point of failure — switching from `run` (which
            // dropped the events on `Err`) closes the live-feedback
            // gap: the dashboard editor SSE channel can render the
            // partial `Text` / `ToolUse` history even when the AI
            // run errored, instead of silently going dark and
            // leaving the operator confused.
            let outcome = agent.run_with_outcome(prompt).await;
            if let Some(e) = &outcome.error {
                tracing::warn!(
                    error = %e,
                    node = %ctx.node,
                    event_count = outcome.events.len(),
                    "ai-agent narration failed; partial events preserved"
                );
            }
            // Surface the text only when the run produced something
            // we can show. Events ride independently so a zero-text
            // failure still carries its activity history.
            let text = if outcome.text.is_empty() && outcome.error.is_some() {
                None
            } else {
                Some(outcome.text)
            };
            Some((text, outcome.events))
        } else {
            None
        };

        // Split out the (text, events) pair so the per-step event
        // array can be surfaced on the terminal slot alongside the
        // text. Pre-projection contract was `{tool, reply}` only;
        // adding `events` is purely additive — REST + MCP consumers
        // that read only `reply` / `tool` keep working. The
        // `agent-event-projection` follow-up
        // (rubix/docs/sessions/data-flow/2026-05-26-data-flow-07-
        // agent-event-projection.md) closes the live-feedback gap
        // for scope 11 §B7 by letting the dashboard editor render
        // per-step `Text` / `ToolUse` activity instead of waiting
        // for the post-commit SSE banner.
        let (reply, agent_events): (Option<String>, Vec<starter_spi::ai::Event>) = match reply {
            Some((text, events)) => (text, events),
            None => (None, Vec::new()),
        };
        let events_value: Option<serde_json::Value> = if agent_events.is_empty() {
            None
        } else {
            serde_json::to_value(&agent_events).ok()
        };

        let body = match (tool_value, reply, events_value) {
            (Some(t), Some(r), Some(ev)) => json!({ "tool": t, "reply": r, "events": ev }),
            (Some(t), Some(r), None) => json!({ "tool": t, "reply": r }),
            (Some(t), None, Some(ev)) => json!({ "tool": t, "events": ev }),
            (Some(t), None, None) => json!({ "tool": t }),
            (None, Some(r), Some(ev)) => json!({ "reply": r, "events": ev }),
            (None, Some(r), None) => json!({ "reply": r }),
            (None, None, Some(ev)) => json!({ "events": ev }),
            (None, None, None) => {
                // Both deterministic and narration paths produced no
                // output. Rather than fail the flow run (silent null
                // body for the caller), return a structured stub so
                // the caller can see *what* went wrong. Common cause:
                // narration disabled + the flow's primary tool needs
                // arguments the zero-arg MCP call didn't supply.
                let hint = primary_tool_name
                    .map(|t| format!("primary tool `{t}` produced no output and narration is disabled"))
                    .unwrap_or_else(|| "no primary tool mapped and narration is disabled".to_owned());
                json!({ "reply": hint, "tool": null })
            }
        };

        let mut out = SlotMap::new();
        out.insert(
            rubix_flows::DEFAULT_OUTPUT_SLOT.to_owned(),
            SlotValue::Json(body),
        );
        Ok(out)
    }
}

/// Conventional tenant id used in single-tenant / laptop dev
/// deployments. The bundled seed data
/// (`rubix.dashboard.disk-overview`, etc.) is written under this
/// tenant, and the rubix login flow does not yet bind sessions to
/// a tenant (see `crates/starter-auth-users/src/routes/login.rs`).
/// Falling back to this when `Principal.tenant_id` is `None` lets
/// the chat surface author dashboards out of the box; the value
/// stays a single source of truth so a follow-up that wires real
/// tenant binding only has to remove the fallback.
const DEFAULT_TENANT: &str = "system";

/// Merge the authenticated session principal (tenant / owner /
/// created_by) into `input` when those fields are absent. Returns
/// `input` unchanged outside an HTTP MCP dispatch (no principal
/// bound), or when the caller already provided values, or when the
/// payload is not a JSON object. See the call site in
/// `RubixAiAgentNode::invoke` for the rationale.
fn augment_tool_input_with_principal(input: Value) -> Value {
    let Some(principal) = starter_mcp::current_principal() else {
        return input;
    };
    let mut value = input;
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    if !obj.contains_key("tenant_id") {
        let tid = principal.tenant_id.as_deref().unwrap_or(DEFAULT_TENANT);
        obj.insert("tenant_id".to_owned(), json!(tid));
    }
    let subject = principal.subject.as_str();
    if !subject.is_empty() {
        obj.entry("owner_principal".to_owned())
            .or_insert_with(|| json!(subject));
        obj.entry("created_by".to_owned())
            .or_insert_with(|| json!(subject));
    }
    value
}

