//! `POST /api/v1/chat/stream` — SSE wire for the chat UX. Per
//! `rubix/docs/sessions/2026-05-25-dashboards-sidebar-sse-and-chat-gaps.md`
//! §"Part 3 — Chat streaming", the underlying [`AiRunner`] machinery
//! has always emitted per-chunk `Event`s into an [`mpsc::Sender`];
//! the v0 [`starter_ai_agent::AgentLoop`] used by
//! [`super::super::boot::mcp::agent_node::RubixAiAgentNode`] just
//! dropped the receiver. This route is the bridge that finally
//! forwards those frames over SSE so the chat bubble can fill
//! character-by-character.
//!
//! Wire shape (one default `data:` frame per `Event`):
//!
//!   - `{"type":"connected","model":"…?"}`            — runner attached.
//!   - `{"type":"text","delta":"…"}`                 — assistant text chunk.
//!   - `{"type":"tool_use","id":"…","name":"…",       — model dispatched a host tool
//!                          "input":…}`                  through the MCP bridge.
//!   - `{"type":"done","input_tokens":…,              — turn finished (totals from
//!                     "output_tokens":…,                the runner; cost in USD).
//!                     "cost_usd":…,
//!                     "duration_ms":…}`
//!   - `{"type":"error","message":"…"}`              — runner-side failure.
//!
//! The route deliberately bypasses the flow engine. The chat tab
//! is a direct conversation with the LLM, primed with the matching
//! rubix skill playbook; it does not need (or want) the per-flow
//! seed-adapter / output-adapter / `FlowAsTool` envelope that the
//! `tools/call`-on-a-flow path uses. This also sidesteps the
//! "channel drop" problem documented in the session note — the
//! `mpsc::Receiver` we hand the runner is owned by the SSE response
//! body, not a [`tokio::spawn`]-and-forget drain.
//!
//! **Tool dispatch via MCP** is opt-in via two environment
//! variables, read once at startup and threaded into the handler
//! state. When both are set, the route writes them into
//! [`CliCfg::mcp_url`] / [`CliCfg::mcp_token`] so the Claude CLI
//! wrapper attaches to the host's own `/api/v1/mcp` endpoint and
//! the model can call rubix tools mid-turn. When either is unset
//! the route still streams text — the chat just behaves as a
//! narration-only assistant. Future work auto-derives the URL from
//! the agent's bind address and bakes a service-token mechanism so
//! operators do not need to copy-paste a bearer.
//!
//! AuthN mirrors [`super::dashboard_events`]: the route is mounted
//! under `with_principal`; an anonymous POST gets a 401 before any
//! stream is opened. CSRF gating mirrors the other event-stream
//! routes (event-stream responses cannot ride CSRF-protected paths
//! for `EventSource`-shaped consumers; we kept the same posture
//! here so the rubix frontend's fetch+reader uses the same gating
//! contract as the sidebar SSE).

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Extension, Json};
use futures::stream::Stream;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt as _;

use starter_spi::ai::{
    AiRunner, Cancel, CliCfg, Event, EventKind, PermissionMode, Provider, RunnerInput, SessionId,
};
use starter_spi::auth::Principal;

use crate::routes::stream_frames::{frame_to_sse, StreamFrame};
use crate::routes::{RouteMeta, RouteRegistrar};

/// Default skill id used when the chat request does not specify
/// one. Picked to be the goal-1 dashboard-builder so a brand-new
/// chat surface lands with the playbook the rubix demo expects.
/// Operators can override per-request via the `skill_id` body field.
const DEFAULT_SKILL_ID: &str = "com.rubix.dashboard-builder";

/// State threaded into the SSE handler.
#[derive(Clone)]
pub struct ChatStreamState {
    /// The single host-built runner (today: Claude CLI or the
    /// fixture-replay runner). Cloned per request — the runner
    /// itself is stateless, so concurrent chats share it freely.
    pub runner: Arc<dyn AiRunner>,
    /// `RUBIX_SERVICE_MCP_URL` value snapshotted at boot. `None`
    /// when unset; the runner then sees `CliCfg.mcp_url = None`
    /// and the model has no host-tool catalogue (narration only).
    pub mcp_url: Option<String>,
    /// `RUBIX_SERVICE_MCP_TOKEN` value snapshotted at boot. `None`
    /// when unset; only meaningful when `mcp_url` is also `Some`.
    pub mcp_token: Option<String>,
}

impl ChatStreamState {
    /// Build a [`ChatStreamState`] reading the two MCP-wiring env
    /// vars once. Empty strings are treated as unset so a stale
    /// `export RUBIX_SERVICE_MCP_URL=` in a shell session does not
    /// accidentally enable the MCP path.
    pub fn from_env(runner: Arc<dyn AiRunner>) -> Self {
        let snap = |key: &str| std::env::var(key).ok().filter(|s| !s.trim().is_empty());
        Self {
            runner,
            mcp_url: snap("RUBIX_SERVICE_MCP_URL"),
            mcp_token: snap("RUBIX_SERVICE_MCP_TOKEN"),
        }
    }
}

/// Build the registrar. The route already carries its full
/// `/api/v1` prefix.
pub fn registrar(state: ChatStreamState) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::POST,
        "/api/v1/chat/stream",
        post(chat_stream).with_state(state),
        RouteMeta::new()
            .describe("SSE bridge from the configured AI runner to the chat UX.")
            .tag("dashboard"),
    )
}

/// Backwards-compatible alias for tests that expect an
/// `axum::Router`.
pub fn router(state: ChatStreamState) -> axum::Router {
    registrar(state).into_router()
}

/// Inbound request body. Both fields are optional so the chat
/// surface can degrade to "just narrate" when the operator hits
/// Send on an empty input.
#[derive(Debug, Deserialize, Default)]
pub struct ChatRequest {
    /// Free-form user message. Empty string is allowed — the
    /// resulting prompt to the LLM is the skill body alone, which
    /// is a useful "introduce yourself" affordance.
    #[serde(default)]
    pub prompt: String,
    /// Reverse-DNS rubix skill id (e.g. `com.rubix.dashboard-builder`).
    /// `None` falls back to [`DEFAULT_SKILL_ID`].
    #[serde(default)]
    pub skill_id: Option<String>,
}

/// Outbound SSE frame is the shared
/// [`StreamFrame`](crate::routes::stream_frames::StreamFrame) —
/// chat uses the `connected` / `text` / `tool_use` /
/// `done(chat keys)` / `error` subset of variants.
async fn chat_stream(
    State(state): State<ChatStreamState>,
    principal: Option<Extension<Principal>>,
    Json(body): Json<ChatRequest>,
) -> axum::response::Response {
    // -- 1. AuthN gate. `with_principal` populates the extension
    //       when authentication succeeds; bail out cleanly otherwise
    //       so we never open a stream for an anonymous client.
    let Some(Extension(principal)) = principal else {
        return (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    };

    // -- 2. Resolve the skill body. Falls back to "no preamble"
    //       when the hint is missing or unresolvable rather than
    //       failing the request — the chat still works without a
    //       skill, just without rubix-specific priming.
    let skill_id = body
        .skill_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_SKILL_ID);
    let skill_body = skill_body_for_hint(skill_id).unwrap_or_default();

    // -- 3. Compose the prompt. The skill body is prepended as a
    //       system-style preamble; the user message follows it
    //       behind a visible separator so the model can tell them
    //       apart even though the CLI flattens both into one
    //       prompt string (see `AgentLoop::call`).
    let prompt = if skill_body.is_empty() {
        body.prompt.clone()
    } else {
        format!(
            "# Skill instructions (follow these)\n\n{skill_body}\n\n---\n\n# User message\n\n{}",
            body.prompt
        )
    };

    // -- 4. CLI config. The runner trait is provider-agnostic but
    //       in practice we only run the Claude CLI wrapper here;
    //       the REST-shape `AgentLoop::call` path is for in-flow
    //       turn loops, not interactive chat. Threading the
    //       `mcp_url` + `mcp_token` into `CliCfg` is what gives the
    //       wrapper an MCP server to attach to so the model can
    //       call rubix tools (D-F5.6).
    let cli = CliCfg {
        prompt,
        // We do not duplicate the skill body in `system_prompt`
        // because the CLI runner's `system_prompt` arg is provider-
        // specific and not honoured uniformly. The combined prompt
        // is the portable seam.
        system_prompt: None,
        mcp_url: state.mcp_url.clone(),
        mcp_token: state.mcp_token.clone(),
        // Restrict the wrapped Claude CLI to MCP-bridged tools
        // only. Without this, the binary's built-in catalogue
        // (`Bash`, `Read`, `AskUserQuestion`, ...) is in scope and
        // the model defaults to `AskUserQuestion`, turning "make
        // me an iot dashboard" into a multi-turn survey instead
        // of an action. `mcp__rubix__*` matches every rubix tool
        // exposed via the MCP server we configure in
        // `crates/starter-ai/src/runners/claude.rs` (server name
        // hard-coded to `"rubix"` there).
        allowed_tools: state.mcp_url.as_ref().map(|_| "mcp__rubix__*".to_owned()),
        // The host has already gated the request at the HTTP layer
        // (login cookie + `with_principal`). Without `Bypass` the
        // CLI's `--permission-mode bypassPermissions` alone still
        // gates MCP tool calls behind a per-call prompt the
        // headless wrapper never answers \u2014 the model emits
        // "Waiting on permission ..." and the call never reaches
        // the MCP server. The Claude runner pairs `Bypass` with
        // `--dangerously-skip-permissions` to also bypass MCP
        // prompts (see `runners/claude.rs`).
        permission_mode: state.mcp_url.as_ref().map(|_| PermissionMode::Bypass),
        ..CliCfg::default()
    };

    // -- 5. Event channel. The runner pushes Events into `tx`; we
    //       wrap `rx` in a `ReceiverStream` that the SSE body
    //       polls. Capacity 32 swallows the small bursts Claude
    //       emits per turn without back-pressuring the runner.
    let (tx, rx) = mpsc::channel::<Event>(32);

    // -- 6. Spawn the runner. The session id encodes the caller's
    //       principal so per-session resume (when CLI resume_id
    //       support lands) honours the auth identity. We do not
    //       persist sessions yet — chat is single-turn today.
    let runner = state.runner.clone();
    let session_id: SessionId = format!("chat:{}", principal.subject).into();
    tokio::spawn(async move {
        // `NoopCancel` keeps the loop running until the runner
        // returns or the client disconnects. When the client
        // disconnects, axum drops the response body, which drops
        // `rx`, which causes the next `tx.send` to error and the
        // runner's stream-loop to bail out naturally — no manual
        // cancellation needed for the v0 chat path.
        let cancel = NoopCancel;
        if let Err(e) = runner
            .run(RunnerInput::Cli(cli), session_id, tx.clone(), &cancel)
            .await
        {
            // Surface runner errors as an in-band `error` frame so
            // the chat UI can render them inline (instead of as a
            // mysterious "stream closed early"). Best-effort —
            // `tx.send` may fail if the client already left.
            let _ = tx
                .send(Event {
                    session_id: starter_spi::ai::SessionId::from("chat:error"),
                    provider: runner.provider().to_string(),
                    kind: EventKind::Error {
                        message: format!("runner failed: {e}"),
                    },
                })
                .await;
        }
    });

    // -- 7. Translate runner Events to ChatFrames and serialise as
    //       SSE default `data:` frames. KeepAlive matches the
    //       sidebar SSE route so reverse proxies do not cull the
    //       stream during a long-running tool dispatch.
    let stream = build_stream(rx);
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

/// Compose the runner event channel into an SSE item stream.
/// Factored out so the unit tests below can drive it without
/// standing up an HTTP layer.
fn build_stream(
    rx: mpsc::Receiver<Event>,
) -> impl Stream<Item = Result<SseEvent, Infallible>> + Send + 'static {
    ReceiverStream::new(rx).map(|ev| frame_to_sse(&event_to_frame(&ev)))
}

/// Project a runner [`Event`] into the flatter wire shape the chat
/// UI consumes. Provider id rides as the SSE event name in a future
/// revision; for now everything lands on the default `message`
/// channel so the `EventSource`-shaped reader on the frontend stays
/// dead-simple.
fn event_to_frame(ev: &Event) -> StreamFrame {
    match &ev.kind {
        EventKind::Connected { model } => StreamFrame::Connected {
            model: model.clone(),
        },
        EventKind::Text { content } => StreamFrame::Text {
            delta: content.clone(),
        },
        EventKind::ToolUse { id, name, input } => StreamFrame::ToolUse {
            id: id.clone(),
            name: name.clone(),
            input: input.clone(),
        },
        EventKind::Done {
            input_tokens,
            output_tokens,
            cost_usd,
            duration_ms,
        } => StreamFrame::done_chat(*input_tokens, *output_tokens, *cost_usd, *duration_ms),
        EventKind::Error { message } => StreamFrame::Error {
            message: message.clone(),
        },
    }
}

/// Read a bundled SKILL.md body, stripping the YAML frontmatter.
/// Mirrors the helper of the same name in
/// [`super::super::boot::mcp::register`]; both must move together
/// when the starter-skills loader lands and replaces the direct
/// `rubix_skills::bundled()` lookup with `SkillRegistry::get`.
fn skill_body_for_hint(hint: &str) -> Option<String> {
    let name = hint.strip_prefix("com.rubix.")?;
    let dir = rubix_skills::bundled();
    let file = dir.get_file(format!("{name}/SKILL.md"))?;
    let raw = file.contents_utf8()?;
    let body = strip_frontmatter(raw).trim();
    if body.is_empty() {
        None
    } else {
        Some(body.to_owned())
    }
}

fn strip_frontmatter(src: &str) -> &str {
    let rest = match src.strip_prefix("---\n") {
        Some(rest) => rest,
        None => return src,
    };
    match rest.find("\n---\n") {
        Some(end) => &rest[end + "\n---\n".len()..],
        None => rest,
    }
}

/// Always-open [`Cancel`] impl used by the chat path. The chat is
/// single-turn — the client disconnects when it wants to abort,
/// which collapses the SSE body / `rx` and naturally drains the
/// runner. The richer cancellation seam is for in-flow runs.
struct NoopCancel;

impl Cancel for NoopCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

// Suppress the unused-import warning when the test cfg is off.
#[allow(dead_code)]
fn _provider_type_used(_: &Provider) {}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use starter_spi::ai::SessionId;

    fn evt(kind: EventKind) -> Event {
        Event {
            session_id: SessionId::from("chat:test"),
            provider: "claude".to_string(),
            kind,
        }
    }

    #[test]
    fn strip_frontmatter_handles_present_and_absent_blocks() {
        // Present.
        let src = "---\nid: x\n---\nbody here\n";
        assert_eq!(strip_frontmatter(src), "body here\n");
        // Absent.
        assert_eq!(strip_frontmatter("body only\n"), "body only\n");
        // Unterminated — degrades to "everything after the opening
        // fence" rather than leaking `---` into the model prompt.
        assert_eq!(strip_frontmatter("---\nstuff\n"), "stuff\n");
    }

    #[test]
    fn skill_body_for_hint_resolves_bundled_skill() {
        let body = skill_body_for_hint("com.rubix.dashboard-builder").expect("bundled");
        assert!(body.starts_with("# Dashboard builder"), "body: {body:.80?}");
        // Frontmatter should be stripped.
        assert!(!body.contains("trust: approved"));
    }

    #[test]
    fn skill_body_for_hint_returns_none_for_unknown_skill() {
        assert!(skill_body_for_hint("com.rubix.does-not-exist").is_none());
        // Non-rubix namespaces are never resolved from this
        // bundle.
        assert!(skill_body_for_hint("starter.flow.ai-agent").is_none());
    }

    #[test]
    fn event_to_frame_maps_each_kind() {
        match event_to_frame(&evt(EventKind::Connected {
            model: Some("claude".into()),
        })) {
            StreamFrame::Connected { model } => assert_eq!(model.as_deref(), Some("claude")),
            _ => panic!("expected Connected"),
        }
        match event_to_frame(&evt(EventKind::Text {
            content: "hi".into(),
        })) {
            StreamFrame::Text { delta } => assert_eq!(delta, "hi"),
            _ => panic!("expected Text"),
        }
        match event_to_frame(&evt(EventKind::ToolUse {
            id: Some("t1".into()),
            name: "rubix.dashboard.list".into(),
            input: Some(serde_json::json!({"tenant_id": "system"})),
        })) {
            StreamFrame::ToolUse { id, name, input } => {
                assert_eq!(id.as_deref(), Some("t1"));
                assert_eq!(name, "rubix.dashboard.list");
                assert_eq!(input, Some(serde_json::json!({"tenant_id": "system"})));
            }
            _ => panic!("expected ToolUse"),
        }
        match event_to_frame(&evt(EventKind::Done {
            duration_ms: 100,
            cost_usd: 0.001,
            input_tokens: 10,
            output_tokens: 20,
        })) {
            StreamFrame::Done {
                input_tokens,
                output_tokens,
                cost_usd,
                duration_ms,
                ..
            } => {
                assert_eq!(input_tokens, Some(10));
                assert_eq!(output_tokens, Some(20));
                assert!((cost_usd.unwrap() - 0.001).abs() < 1e-9);
                assert_eq!(duration_ms, Some(100));
            }
            _ => panic!("expected Done"),
        }
        match event_to_frame(&evt(EventKind::Error {
            message: "boom".into(),
        })) {
            StreamFrame::Error { message } => assert_eq!(message, "boom"),
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn frame_to_sse_serialises_to_default_event() {
        let frame = StreamFrame::Text { delta: "hi".into() };
        let sse = frame_to_sse(&frame).expect("infallible");
        let debug = format!("{sse:?}");
        assert!(debug.contains(r#"\"type\":\"text\""#), "{debug}");
        assert!(debug.contains(r#"\"delta\":\"hi\""#), "{debug}");
    }

    #[tokio::test]
    async fn build_stream_forwards_events_in_order() {
        let (tx, rx) = mpsc::channel::<Event>(4);
        tx.send(evt(EventKind::Connected { model: None }))
            .await
            .unwrap();
        tx.send(evt(EventKind::Text {
            content: "hel".into(),
        }))
        .await
        .unwrap();
        tx.send(evt(EventKind::Text {
            content: "lo".into(),
        }))
        .await
        .unwrap();
        tx.send(evt(EventKind::Done {
            duration_ms: 1,
            cost_usd: 0.0,
            input_tokens: 0,
            output_tokens: 0,
        }))
        .await
        .unwrap();
        drop(tx);

        let mut s = Box::pin(build_stream(rx));
        let mut frames = Vec::new();
        while let Some(item) = s.next().await {
            let sse = item.expect("Infallible");
            frames.push(format!("{sse:?}"));
        }
        assert_eq!(frames.len(), 4);
        assert!(frames[0].contains("connected"));
        assert!(frames[1].contains(r#"\"delta\":\"hel\""#), "{}", frames[1]);
        assert!(frames[2].contains(r#"\"delta\":\"lo\""#), "{}", frames[2]);
        assert!(frames[3].contains("done"));
    }

    #[test]
    fn chat_state_from_env_reads_mcp_vars() {
        struct StubRunner;
        #[async_trait::async_trait]
        impl AiRunner for StubRunner {
            fn provider(&self) -> &Provider {
                &Provider::Claude
            }
            async fn ready(&self) -> bool {
                true
            }
            async fn run(
                &self,
                _input: RunnerInput,
                _session: SessionId,
                _on_event: mpsc::Sender<Event>,
                _cancel: &dyn Cancel,
            ) -> Result<starter_spi::ai::RunResult, starter_spi::ai::RunnerError> {
                Ok(starter_spi::ai::RunResult::default())
            }
        }

        // Snapshot + restore the env so this test doesn't leak.
        let old_url = std::env::var("RUBIX_SERVICE_MCP_URL").ok();
        let old_tok = std::env::var("RUBIX_SERVICE_MCP_TOKEN").ok();

        std::env::set_var("RUBIX_SERVICE_MCP_URL", "http://127.0.0.1:8088/api/v1/mcp");
        std::env::set_var("RUBIX_SERVICE_MCP_TOKEN", "tok123");
        let s = ChatStreamState::from_env(Arc::new(StubRunner));
        assert_eq!(
            s.mcp_url.as_deref(),
            Some("http://127.0.0.1:8088/api/v1/mcp")
        );
        assert_eq!(s.mcp_token.as_deref(), Some("tok123"));

        // Empty string is treated as unset.
        std::env::set_var("RUBIX_SERVICE_MCP_URL", "");
        let s = ChatStreamState::from_env(Arc::new(StubRunner));
        assert!(s.mcp_url.is_none());

        // Restore.
        match old_url {
            Some(v) => std::env::set_var("RUBIX_SERVICE_MCP_URL", v),
            None => std::env::remove_var("RUBIX_SERVICE_MCP_URL"),
        }
        match old_tok {
            Some(v) => std::env::set_var("RUBIX_SERVICE_MCP_TOKEN", v),
            None => std::env::remove_var("RUBIX_SERVICE_MCP_TOKEN"),
        }
    }
}
