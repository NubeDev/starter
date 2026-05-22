//! `POST /api/builder/stream` — live page-builder SSE route.
//!
//! See `examples/flow-agent/PAGE-BUILDER-LIVE.md` (parent SCOPE) and
//! `PAGE-BUILDER-LIVE-BACKEND.md` (session scope) for the contract.
//!
//! Demo lane: tool def, validator, system prompt, KIND_ALLOW are all
//! inline. They graduate to `starter-ai-sdui-tool` post-demo.
//!
//! Wire shape (SSE, one JSON per `data:` line):
//! ```text
//! data: {"type":"status","phase":"thinking","message":"Asking Claude…"}
//! data: {"type":"status","phase":"writing"}
//! data: {"type":"full-render","tree":{...}}
//! data: {"type":"status","phase":"done"}
//! ```
//!
//! On failure exactly **one** `{"type":"error","error":"…"}` frame is
//! emitted before the stream closes — never both `error` and
//! `status: error`.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::stream::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;

use starter_ai::{Registry, TokenCancel};
use starter_spi::ai::{
    CliCfg, Event as AiEvent, EventKind, HistoryMessage, Provider, RestCfg, RunnerInput, SessionId,
    ToolChoice, ToolDef,
};

use crate::rest::RestState;

/// Allowed SDUI `type` discriminants — single source of truth for the
/// demo lane. Must match `packages/starter-sdui-react/src/registry/
/// types.ts`'s `Kind` union; enforced by [`tests::kind_allow_matches_ts_union`].
const KIND_ALLOW: &[&str] = &[
    "page",
    "row",
    "col",
    "grid",
    "stack",
    "tabs",
    "card",
    "text",
    "heading",
    "badge",
    "kpi",
    "kpi_grid",
    "button",
    "link",
    "table",
    "form",
    "field",
    "select",
    "toggle",
    "custom",
    "chart",
    "sparkline",
    "tree",
    "timeline",
    "markdown",
    "code",
    "wizard",
    "drawer",
    "rich_text",
    "diff",
    "ref_picker",
    "date_range",
];

const MAX_DEPTH: usize = 12;
const MAX_WIDTH: usize = 64;
const MAX_ID_LEN: usize = 64;
/// Documented per SCOPE §4.8 / L7. CLI runners don't expose a
/// `max_tokens` knob (the `claude` binary picks its own); the REST
/// runner does and we set it on the `RestCfg` below.
const MAX_TOKENS: u32 = 8192;
/// Wall-clock budget for the whole runner call. Haiku via the CLI is
/// usually 8-25 s; complex dashboards can graze 30 s. 60 s gives us
/// headroom without making errors invisible.
const WALL_CLOCK: Duration = Duration::from_secs(60);
const TOOL_NAME: &str = "emit_ui_tree";
/// Transcript replay tail (MEMORY.md M-C). The builder route
/// pre-fetches the last N turns of the session and hands them to the
/// runner as conversation history so follow-up prompts like "undo
/// that" or "make the button blue instead" land in context. Snapshot
/// alone is not enough — it tells the model what the page looks like,
/// not what the user previously asked. 20 turns ≈ 10 exchanges,
/// comfortably under the model context budget for haiku/sonnet.
const TRANSCRIPT_REPLAY_LIMIT: usize = 20;

const SYSTEM_PROMPT: &str = include_str!("builder_system_prompt.txt");

/// System prompt for `mode="ask"`. The model answers the user's
/// question conversationally. It must NOT emit a tree or JSON — the
/// frontend renders the reply as a chat bubble. Kept short so the
/// build-prompt's schema bias doesn't leak in (we don't include
/// `SYSTEM_PROMPT` here).
const ASK_SYSTEM_PROMPT: &str = include_str!("builder_ask_prompt.txt");

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BuilderRequest {
    /// Free-form description of the page to generate (1..=4000 chars).
    /// Optional at the type level so we can emit a 400 (not 422) when
    /// callers omit the field; validated below.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Provider id. Demo lane accepts `"claude"` (default) and
    /// `"anthropic"`; the runtime picks the REST runner whenever
    /// `ANTHROPIC_API_KEY` is set, falling back to CLI.
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Conversation mode. `"build"` (default) generates / edits the
    /// SDUI tree and emits a `full-render` frame. `"ask"` answers a
    /// question conversationally and emits a single `message` frame
    /// without touching the tree — use this for clarifying questions
    /// like "can we add fake data?" or "what does this component do?".
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Optional agent-session id (UUIDv7 string). When provided, the
    /// generated tree is persisted as an artifact under this session
    /// (MEMORY.md Phase M-D). When absent, the request stays
    /// stateless (MEMORY.md M13 — "ephemeral; never persisted").
    #[serde(default)]
    pub session_id: Option<String>,
    /// Artifact key to seed the request with as a "previous state"
    /// snapshot (typically `"tree"`). Only honored when `session_id`
    /// is set; ignored otherwise. The snapshot is prepended to the
    /// system prompt so the model can edit/extend the prior tree
    /// instead of regenerating from scratch.
    #[serde(default)]
    pub include_artifact: Option<String>,
}

fn default_provider() -> String {
    "claude".to_owned()
}

fn default_mode() -> String {
    "build".to_owned()
}

/// Which conversational lane this turn is on. `Build` runs the tree
/// generator and validator. `Ask` runs a Q&A turn — no tree, no
/// validator, one `message` frame back to the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuilderMode {
    Build,
    Ask,
}

impl BuilderMode {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "build" | "edit" => Some(Self::Build),
            "ask" => Some(Self::Ask),
            _ => None,
        }
    }
}

/// `POST /api/builder/stream`. Returns either:
/// - `400` on a malformed body / unknown provider,
/// - `503` (with `Retry-After: 0`) when the requested provider is
///   compiled in but not currently `ready()`,
/// - `200` SSE stream of `BuilderEvent` frames otherwise.
#[utoipa::path(post, path = "/api/builder/stream", tag = "builder",
    request_body = BuilderRequest,
    responses(
        (status = 200, description = "SSE stream of BuilderEvent frames",
            content_type = "text/event-stream"),
        (status = 400, description = "Validation failed"),
        (status = 503, description = "Provider compiled in but not ready"),
    ))]
pub async fn builder_stream(
    State(state): State<RestState>,
    Json(req): Json<BuilderRequest>,
) -> Response {
    let prompt = req.prompt.unwrap_or_default().trim().to_owned();
    if prompt.is_empty() || prompt.chars().count() > 4000 {
        return bad_request("prompt must be 1..=4000 chars");
    }
    if req.provider != "claude" && req.provider != "anthropic" {
        return bad_request(&format!("unknown provider `{}`", req.provider));
    }
    let builder_mode = match BuilderMode::parse(req.mode.trim()) {
        Some(m) => m,
        None => {
            return bad_request(&format!(
                "unknown mode `{}` (expected `build` or `ask`)",
                req.mode
            ));
        }
    };

    // Parse session id up front so a malformed value becomes 400
    // (not a silent stateless fallback). The empty string `""` is
    // tolerated as "no session" so frontends can send the same shape
    // either way.
    let session_id = match req.session_id.as_deref().map(str::trim) {
        Some("") | None => None,
        Some(s) => match starter_flow_spi::agent_session::AgentSessionId::parse(s) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("invalid session_id"),
        },
    };
    let include_artifact_key = req.include_artifact.unwrap_or_else(|| "tree".to_owned());

    // Snapshot pre-fetch (MEMORY.md M-D, Snapshot replay strategy).
    // We do this synchronously before spawning so a malformed
    // snapshot can be surfaced as 500 immediately; the SSE stream
    // only opens once we know the back-fill succeeded (or there was
    // nothing to back-fill).
    let snapshot = match session_id {
        Some(id) => {
            match state
                .agent_sessions
                .latest_artifact(id, &include_artifact_key)
                .await
            {
                Ok(art) => art.map(|a| (include_artifact_key.clone(), a.value)),
                Err(e) => {
                    tracing::error!(error = %e, "snapshot fetch failed");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "snapshot fetch failed" })),
                    )
                        .into_response();
                }
            }
        }
        None => None,
    };

    // Transcript replay (MEMORY.md M-C). Pull the tail of the prior
    // turns so the model has conversational context for follow-ups.
    // Persist failures here are non-fatal: snapshot + system prompt
    // still produce a usable single-shot response, and an error
    // crossing the wire would mask the actual failure mode.
    let history = match session_id {
        Some(id) => match fetch_history(&state.agent_sessions, id, TRANSCRIPT_REPLAY_LIMIT).await {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!(error = %e, "transcript replay fetch failed, continuing without");
                Vec::new()
            }
        },
        None => Vec::new(),
    };

    // Prefer the REST runner (`Provider::Anthropic`) when an API key
    // is set: the CLI runner (`Provider::Claude`) does not surface
    // tools through CliCfg, so the model cannot emit a structured
    // `ToolUse` and the route falls back to prose-rescue. See
    // PAGE-BUILDER-LIVE.md §0 (P0 result).
    let (resolved, mode) = match resolve_runner(state.ai.registry()).await {
        Some(pair) => pair,
        None => {
            return provider_unavailable(
                "no AI runner available: export ANTHROPIC_API_KEY (preferred) or \
                 install Claude Code (`claude` on PATH + `claude auth login`)",
            );
        }
    };

    let (frame_tx, frame_rx) = mpsc::channel::<String>(16);

    // §4.9 client-abort wiring. The cancel handle is shared with the
    // runner task; a drop-guard on the SSE stream trips it when the
    // client closes the connection. The runner observes the trip
    // either via its `Cancel` future (REST) or by polling between
    // stdout reads (CLI), and aborts the upstream call promptly.
    let cancel = TokenCancel::new();

    // Persistence context for the spawned task. `None` keeps the
    // request stateless (MEMORY.md M13).
    let persist = session_id.map(|id| PersistCtx {
        store: state.agent_sessions.clone(),
        session_id: id,
        artifact_key: include_artifact_key.clone(),
        user_prompt: prompt.clone(),
    });

    tokio::spawn(run_builder(
        resolved,
        mode,
        builder_mode,
        prompt,
        snapshot,
        history,
        persist,
        frame_tx,
        cancel.clone(),
    ));

    let guard = CancelOnDrop {
        cancel: cancel.clone(),
    };
    let stream = ReceiverStream::new(frame_rx).map(move |payload| {
        // Keep the guard alive for as long as the stream is polled;
        // the closure captures it by move and drops with the stream.
        let _keep = &guard;
        Ok::<_, Infallible>(SseEvent::default().data(payload))
    });

    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    );

    // Belt-and-braces headers per §4.4 (L9 anti-buffering). axum's Sse
    // sets `text/event-stream` + `Cache-Control: no-cache` already;
    // the explicit no-transform + X-Accel-Buffering pin the rest so
    // upstream proxies (nginx, vite dev) don't buffer the stream.
    let mut resp = sse.into_response();
    let h = resp.headers_mut();
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-transform"),
    );
    h.insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));
    h.insert("x-accel-buffering", HeaderValue::from_static("no"));
    resp
}

/// Drop-guard that trips the runner's `Cancel` when the SSE response
/// stream is dropped (client disconnect, route unmount).
struct CancelOnDrop {
    cancel: TokenCancel,
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Which runner shape we're driving for this request. The CLI path
/// is best-effort (P0 documented it generally returns prose, not a
/// structured `ToolUse`); REST is the production path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunnerMode {
    Rest,
    Cli,
}

/// Pick a ready runner, preferring the REST path (Anthropic). Returns
/// `None` when neither is registered + ready, which becomes a 503 to
/// the caller.
async fn resolve_runner(
    registry: &Registry,
) -> Option<(Arc<dyn starter_spi::ai::AiRunner>, RunnerMode)> {
    if let Some(r) = registry.get(&Provider::Anthropic) {
        if r.ready().await {
            return Some((r, RunnerMode::Rest));
        }
    }
    if let Some(r) = registry.get(&Provider::Claude) {
        if r.ready().await {
            return Some((r, RunnerMode::Cli));
        }
    }
    None
}

// ---------------------------------------------------------------------
// SSE producer
// ---------------------------------------------------------------------

/// Side-channel context for persisting a successful generation to an
/// agent session (MEMORY.md Phase M-D). Built only when the caller
/// supplies a `session_id`; absent for stateless requests.
struct PersistCtx {
    store: Arc<dyn starter_flow_spi::agent_session::AgentSessionStore>,
    session_id: starter_flow_spi::agent_session::AgentSessionId,
    artifact_key: String,
    /// Echoed back to the SSE stream as `session_artifact` so the
    /// surface can sync its local state to the persisted version
    /// without a follow-up GET.
    user_prompt: String,
}

// One extra argument (history) over the clippy threshold of 7.
// Keeping a single fn is clearer than packaging the parameters into
// a context struct that's only ever constructed and consumed once.
#[allow(clippy::too_many_arguments)]
async fn run_builder(
    runner: Arc<dyn starter_spi::ai::AiRunner>,
    mode: RunnerMode,
    builder_mode: BuilderMode,
    prompt: String,
    snapshot: Option<(String, JsonValue)>,
    history: Vec<HistoryMessage>,
    persist: Option<PersistCtx>,
    tx: mpsc::Sender<String>,
    cancel: TokenCancel,
) {
    // Frame 1 — thinking. Sent immediately so the client renders a
    // spinner within ~ms of accepting the connection.
    let _ = tx
        .send(
            json!({
                "type": "status",
                "phase": "thinking",
                "message": "Asking Claude…",
            })
            .to_string(),
        )
        .await;

    let (ai_tx, mut ai_rx) = mpsc::channel::<AiEvent>(32);
    let session = SessionId::from(format!("builder-{}", short_id()));

    // Compose the system prompt: base prompt + optional snapshot
    // (MEMORY.md Snapshot replay). Keeping the snapshot fenced and
    // labelled stops the model from treating it as instructions; it
    // gets read as "this is what the page looks like right now,
    // edit it".
    let system_prompt = match snapshot.as_ref() {
        None => SYSTEM_PROMPT.to_owned(),
        Some((key, value)) => format!(
            "{SYSTEM_PROMPT}\n\n# Previous `{key}` artifact (edit this in place)\n```json\n{}\n```\n",
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        ),
    };

    // The Ask lane composes its own system prompt from a different
    // base; the snapshot (if any) is still useful context but the
    // build-mode SDUI-builder rules and few-shots aren't relevant.
    let ask_system_prompt = match snapshot.as_ref() {
        None => ASK_SYSTEM_PROMPT.to_owned(),
        Some((key, value)) => format!(
            "{ASK_SYSTEM_PROMPT}\n\n# Current `{key}` (read-only — you cannot edit it from Ask mode)\n```json\n{}\n```\n",
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        ),
    };

    let input = match (mode, builder_mode) {
        (RunnerMode::Rest, BuilderMode::Build) => {
            // REST runner — pass the tool def + force the call. This is
            // the path that reliably returns a structured ToolUse.
            //
            // `history` carries the prior conversation (MEMORY.md M-C
            // Transcript replay). The current prompt is sent as the
            // top-level `prompt` field; the runner appends it as the
            // final user message after `history`.
            RunnerInput::Rest(RestCfg {
                prompt: prompt.clone(),
                system_prompt: Some(system_prompt.clone()),
                history: history.clone(),
                tools: vec![tool_def()],
                tool_choice: Some(ToolChoice::Tool {
                    name: TOOL_NAME.to_owned(),
                }),
                max_tokens: Some(MAX_TOKENS),
                ..RestCfg::default()
            })
        }
        (RunnerMode::Rest, BuilderMode::Ask) => {
            // Ask lane on REST — no tools, plain prose reply. The
            // model writes one text message; we surface it as a
            // `message` SSE frame.
            RunnerInput::Rest(RestCfg {
                prompt: prompt.clone(),
                system_prompt: Some(ask_system_prompt.clone()),
                history: history.clone(),
                tools: vec![],
                tool_choice: None,
                max_tokens: Some(MAX_TOKENS),
                ..RestCfg::default()
            })
        }
        (RunnerMode::Cli, BuilderMode::Build) => {
            // CLI runners don't surface tools through CliCfg; fold the
            // schema into the system prompt as a best-effort. Per P0
            // the model usually replies with prose JSON on this path,
            // which the resolver catches as a single `error` frame.
            //
            // The CLI shape has no native conversation channel either,
            // so prior turns are inlined into the system prompt under
            // a fenced section. Keep it short — the CLI lane is the
            // fallback, not the primary surface.
            //
            // Pin `haiku` (≈3-5× faster than the default opus) so a
            // CLI cold-call comfortably fits inside `WALL_CLOCK`. The
            // task is structured-JSON emission against a fixed schema;
            // the smaller model is plenty smart and the latency win is
            // what keeps the demo lane usable.
            RunnerInput::Cli(CliCfg {
                prompt: prompt.clone(),
                system_prompt: Some(format!(
                    "{system_prompt}{transcript}\n\n\
                     # CLI lane — no tool API\n\
                     Your environment has NO tool-calling surface. Do not\n\
                     attempt to call `{TOOL_NAME}` or any other tool, do not\n\
                     say the tool is unavailable, do not ask the user for\n\
                     clarification, and do not emit prose, markdown, or code\n\
                     fences. Reply with a single JSON object matching the\n\
                     schema below — the JSON object IS your entire reply.\n\
                     \n\
                     # JSON schema (the shape your reply must match)\n\
                     {schema}\n",
                    transcript = format_history_for_cli(&history),
                    schema =
                        serde_json::to_string_pretty(&tool_def().input_schema).unwrap_or_default()
                )),
                model: Some("haiku".into()),
                // Disable every built-in tool (Bash, Read, Write,
                // Edit, …). The builder route only wants prose JSON
                // back; without this the model occasionally takes
                // "iot dashboard" as a directive to scaffold a
                // project and writes IOT_DASHBOARD_*.md / .json into
                // CWD. With `tools=""` there is nothing it can call,
                // so `permission_mode` becomes moot — the wrapper
                // never reaches the approval gate.
                tools: Some(String::new()),
                ..CliCfg::default()
            })
        }
        (RunnerMode::Cli, BuilderMode::Ask) => {
            // Ask lane on CLI — prose-only reply. Same tool
            // lockdown as Build (no FS side-effects); different
            // system prompt that explicitly asks for conversational
            // text, not JSON.
            RunnerInput::Cli(CliCfg {
                prompt: prompt.clone(),
                system_prompt: Some(format!(
                    "{ask_system_prompt}{transcript}",
                    transcript = format_history_for_cli(&history),
                )),
                model: Some("haiku".into()),
                tools: Some(String::new()),
                ..CliCfg::default()
            })
        }
    };

    // Pump runner Events → (status:writing on first byte, capture
    // tool-use, accumulate prose for the prose-only failure path).
    let writing_tx = tx.clone();
    let pump_cancel = cancel.clone();
    let pump = tokio::spawn(async move {
        let mut writing_sent = false;
        let mut prose = String::new();
        let mut tool_input: Option<JsonValue> = None;
        let mut runner_error: Option<String> = None;

        while let Some(ev) = ai_rx.recv().await {
            if !writing_sent {
                writing_sent = true;
                let _ = writing_tx
                    .send(json!({ "type": "status", "phase": "writing" }).to_string())
                    .await;
            }
            match ev.kind {
                EventKind::Text { content } => prose.push_str(&content),
                EventKind::ToolUse { name, input, .. } if name == TOOL_NAME => {
                    if let Some(v) = input {
                        tool_input = Some(v);
                    }
                }
                EventKind::ToolUse { .. } => {
                    // Other tool calls — ignore; we only care about
                    // the one we asked for.
                }
                EventKind::Error { message } => {
                    runner_error = Some(message);
                }
                EventKind::Connected { .. } | EventKind::Done { .. } => {}
            }

            if tool_input.is_some() {
                // Got what we need — don't wait for the rest of the
                // CLI session to drain.
                pump_cancel.cancel();
            }
        }
        (prose, tool_input, runner_error)
    });

    let run_fut = runner.run(input, session, ai_tx, &cancel);
    let outcome = timeout(WALL_CLOCK, run_fut).await;

    // Pump always completes once the sender is dropped — runner.run
    // returns => ai_tx dropped => recv loop exits.
    let (prose, tool_input, runner_error) = pump.await.unwrap_or_default();

    match builder_mode {
        BuilderMode::Build => {
            match resolve_outcome(outcome, prose, tool_input, runner_error, mode) {
                Ok(tree) => {
                    // Persist before announcing success so a caller
                    // that reads the artifact on `phase: "done"`
                    // always sees the tree that produced this frame.
                    // A persist failure does NOT poison the response:
                    // the tree is still valid for this turn, the user
                    // just won't have history. We log and emit an
                    // out-of-band `session_error` frame so the
                    // surface can degrade gracefully (stay stateless).
                    if let Some(ctx) = &persist {
                        if let Err(e) = persist_turn(ctx, &tree).await {
                            tracing::error!(error = %e, "agent-session persist failed");
                            let _ = tx
                                .send(
                                    json!({
                                        "type": "session_error",
                                        "error": format!("persist failed: {e}"),
                                    })
                                    .to_string(),
                                )
                                .await;
                        } else {
                            let _ = tx
                                .send(
                                    json!({
                                        "type": "session_artifact",
                                        "session_id": ctx.session_id.to_string(),
                                        "key": ctx.artifact_key,
                                    })
                                    .to_string(),
                                )
                                .await;
                        }
                    }
                    let _ = tx
                        .send(json!({ "type": "full-render", "tree": tree }).to_string())
                        .await;
                    let _ = tx
                        .send(json!({ "type": "status", "phase": "done" }).to_string())
                        .await;
                }
                Err(msg) => emit_error(&tx, msg).await,
            }
        }
        BuilderMode::Ask => match resolve_ask_outcome(outcome, prose, runner_error) {
            Ok(message) => {
                if let Some(ctx) = &persist {
                    if let Err(e) = persist_ask_turn(ctx, &message).await {
                        tracing::error!(error = %e, "agent-session persist failed (ask)");
                        let _ = tx
                            .send(
                                json!({
                                    "type": "session_error",
                                    "error": format!("persist failed: {e}"),
                                })
                                .to_string(),
                            )
                            .await;
                    }
                }
                let _ = tx
                    .send(
                        json!({
                            "type": "message",
                            "role": "assistant",
                            "text": message,
                        })
                        .to_string(),
                    )
                    .await;
                let _ = tx
                    .send(json!({ "type": "status", "phase": "done" }).to_string())
                    .await;
            }
            Err(msg) => emit_error(&tx, msg).await,
        },
    }
}

/// Ask-mode resolver. The model's prose IS the answer; there is no
/// tree, no validator. We only need to surface runner-level failures
/// (timeout, transport error) and reject empty replies as upstream
/// errors so the client sees a deterministic terminal frame.
fn resolve_ask_outcome(
    outcome: Result<
        Result<starter_spi::ai::RunResult, starter_spi::ai::RunnerError>,
        tokio::time::error::Elapsed,
    >,
    prose: String,
    runner_error: Option<String>,
) -> Result<String, String> {
    let result = match outcome {
        Err(_elapsed) => {
            return Err(format!("timeout after {}s", WALL_CLOCK.as_secs()));
        }
        Ok(Err(e)) => return Err(format!("runner failed: {e}")),
        Ok(Ok(r)) => r,
    };
    // Combine streamed prose with any final-message text the runner
    // surfaces only on completion (REST tends to emit text via Text
    // events during the stream; CLI varies by wrapper, and some
    // runners only populate `result.text` at the end).
    let mut text = prose.trim().to_owned();
    if text.is_empty() && !result.text.trim().is_empty() {
        text = result.text.trim().to_owned();
    }
    if text.is_empty() {
        if let Some(msg) = result.error.or(runner_error) {
            return Err(format!("upstream error: {msg}"));
        }
        return Err("empty reply".to_owned());
    }
    Ok(text)
}

/// Persist an Ask-mode turn pair: the user question + the
/// assistant's prose reply. No artifact attaches in this lane — the
/// reply text lives in the turn body itself. Same atomicity
/// guarantee as `persist_turn`.
async fn persist_ask_turn(
    ctx: &PersistCtx,
    reply: &str,
) -> Result<(), starter_flow_spi::agent_session::AgentSessionError> {
    use starter_flow_spi::agent_session::{TurnInput, TurnRole};
    ctx.store
        .append_turn_with_artifacts(
            ctx.session_id,
            TurnInput::new(TurnRole::User, JsonValue::String(ctx.user_prompt.clone())),
            &[],
        )
        .await?;
    ctx.store
        .append_turn_with_artifacts(
            ctx.session_id,
            TurnInput::new(TurnRole::Assistant, JsonValue::String(reply.to_owned())),
            &[],
        )
        .await?;
    Ok(())
}

/// Persist a user turn + an `Assistant` turn carrying the freshly
/// generated tree as a versioned artifact. Both writes land in a
/// single backend transaction (the store guarantees atomicity).
async fn persist_turn(
    ctx: &PersistCtx,
    tree: &JsonValue,
) -> Result<(), starter_flow_spi::agent_session::AgentSessionError> {
    use starter_flow_spi::agent_session::{ArtifactWrite, TurnInput, TurnRole};
    // User turn — record the prompt so SummaryPlusTail replay can
    // reconstruct the conversation later (MEMORY.md M-C). No
    // artifacts attach to the user turn; pass an empty slice.
    ctx.store
        .append_turn_with_artifacts(
            ctx.session_id,
            TurnInput::new(TurnRole::User, JsonValue::String(ctx.user_prompt.clone())),
            &[],
        )
        .await?;
    // Assistant turn — bundle the artifact so the read path returns
    // the same `seq` that produced it.
    ctx.store
        .append_turn_with_artifacts(
            ctx.session_id,
            TurnInput::new(TurnRole::Assistant, JsonValue::String(String::new())),
            &[ArtifactWrite::new(&ctx.artifact_key, tree.clone())],
        )
        .await?;
    Ok(())
}

/// Pull the tail of a session's persisted turns and shape them into
/// the runner's `HistoryMessage` form (MEMORY.md M-C Transcript
/// replay). `list_turns` returns oldest-first, so we fetch a soft
/// cap of `2*tail` rows (one user + one assistant per exchange) and
/// trim to the last `tail` turns.
///
/// Assistant turns persist with empty `content` (the SDUI tree lives
/// in the artifact, not the turn body); we substitute a short
/// synthetic marker so the role-alternation the model expects is
/// preserved. The current snapshot is already in the system prompt,
/// so we don't need to inline prior trees here.
async fn fetch_history(
    store: &Arc<dyn starter_flow_spi::agent_session::AgentSessionStore>,
    session_id: starter_flow_spi::agent_session::AgentSessionId,
    tail: usize,
) -> Result<Vec<HistoryMessage>, starter_flow_spi::agent_session::AgentSessionError> {
    use starter_flow_spi::agent_session::TurnRole;

    let fetch_cap = u32::try_from(tail.saturating_mul(2)).unwrap_or(u32::MAX);
    let turns = store.list_turns(session_id, None, Some(fetch_cap)).await?;
    let start = turns.len().saturating_sub(tail);

    let mut out = Vec::with_capacity(turns.len() - start);
    for t in turns.into_iter().skip(start) {
        let role = match t.role {
            TurnRole::User => "user",
            TurnRole::Assistant => "assistant",
            // Tool turns (and any future variant) are not part of
            // the user-facing conversation thread; skip rather than
            // confuse the role-alternation expectation.
            _ => continue,
        };
        let raw = t
            .content
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| t.content.to_string());
        let content = if role == "assistant" && raw.trim().is_empty() {
            "(updated the page tree)".to_owned()
        } else {
            raw
        };
        out.push(HistoryMessage {
            role: role.to_owned(),
            content,
        });
    }
    Ok(out)
}

/// Format the replayed history as a fenced section appended to the
/// CLI runner's system prompt. CLI runners have no native
/// conversation channel so the history is inlined; the section is
/// labelled so the model treats it as context, not instructions.
fn format_history_for_cli(history: &[HistoryMessage]) -> String {
    if history.is_empty() {
        return String::new();
    }
    let mut s = String::from("\n\n# Conversation so far\n");
    for m in history {
        s.push_str(&format!("- **{}**: {}\n", m.role, m.content));
    }
    s
}

/// Collapse the runner outcome + pump captures into a single
/// `Ok(tree) | Err(error_message)` so the caller emits exactly one
/// terminal frame.
fn resolve_outcome(
    outcome: Result<
        Result<starter_spi::ai::RunResult, starter_spi::ai::RunnerError>,
        tokio::time::error::Elapsed,
    >,
    prose: String,
    tool_input: Option<JsonValue>,
    runner_error: Option<String>,
    mode: RunnerMode,
) -> Result<JsonValue, String> {
    let result = match outcome {
        Err(_elapsed) => {
            return Err(format!("timeout after {}s", WALL_CLOCK.as_secs()));
        }
        Ok(Err(e)) => return Err(format!("runner failed: {e}")),
        Ok(Ok(r)) => r,
    };

    // Resolution order is intentional: a captured `tool_input` always
    // wins over a runner-level error. The pump trips `pump_cancel`
    // the moment a complete `emit_ui_tree` call arrives so the CLI
    // session doesn't drain for another ~10 s; that self-induced
    // cancel surfaces as `result.error = Some("cancelled")`, but the
    // payload is real and the request is a success.
    let tool_input = tool_input.or_else(|| {
        result
            .tool_uses
            .into_iter()
            .find(|tu| tu.name == TOOL_NAME)
            .map(|tu| tu.input)
    });

    if let Some(args) = tool_input {
        return parse_and_validate(args).map_err(|reason| format!("invalid tree: {reason}"));
    }

    if let Some(upstream) = result.error.or(runner_error.clone()) {
        return Err(format!("upstream error: {upstream}"));
    }

    if let Some(msg) = runner_error {
        return Err(format!("upstream error: {msg}"));
    }

    // CLI prose-rescue (§0 fallback). The CLI runner has no `tools`
    // surface, so the model usually obeys the system-prompt schema by
    // emitting raw JSON in prose. If that JSON validates as a tree,
    // accept it — we're still inside L1's spirit (one well-formed
    // payload, no streaming, validated against the same schema). The
    // REST path always has a real ToolUse and never reaches this.
    if matches!(mode, RunnerMode::Cli) {
        if let Some(extracted) = extract_first_json_object(prose.as_str()) {
            if let Ok(tree) = parse_and_validate(extracted) {
                return Ok(tree);
            }
        }
    }

    let snippet = first_n_chars(prose.trim(), 200);
    Err(format!(
        "provider returned text instead of tool call: {snippet:?}",
    ))
}

/// Pull the first balanced top-level JSON object out of a string,
/// ignoring everything outside `{ … }`. Handles markdown code fences
/// (```json … ```), leading prose, and trailing prose. Returns the
/// parsed `serde_json::Value` on success.
///
/// The scan tracks brace depth and respects string literals (so braces
/// inside `"…"` don't unbalance the counter). It does NOT validate
/// the JSON beyond what `serde_json::from_str` does.
fn extract_first_json_object(text: &str) -> Option<JsonValue> {
    let bytes = text.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if escape {
            escape = false;
            continue;
        }
        if in_string {
            match b {
                b'\\' => escape = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let candidate = &text[start..=i];
                    return serde_json::from_str::<JsonValue>(candidate).ok();
                }
            }
            _ => {}
        }
    }
    None
}

async fn emit_error(tx: &mpsc::Sender<String>, error: String) {
    let _ = tx
        .send(json!({ "type": "error", "error": error }).to_string())
        .await;
}

fn first_n_chars(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_owned();
    }
    s.chars().take(n).collect::<String>() + "…"
}

fn short_id() -> String {
    // Cheap monotonic id — no uuid dep needed for a session tag.
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}-{n}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    )
}

// ---------------------------------------------------------------------
// HTTP error helpers
// ---------------------------------------------------------------------

fn bad_request(msg: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
}

fn provider_unavailable(hint: &str) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert("retry-after", HeaderValue::from_static("0"));
    (
        StatusCode::SERVICE_UNAVAILABLE,
        headers,
        Json(json!({
            "error": "provider unavailable",
            "hint": hint,
        })),
    )
        .into_response()
}

// ---------------------------------------------------------------------
// Tool def
// ---------------------------------------------------------------------

/// Inline tool def. Schema is intentionally permissive on per-node
/// fields (the runtime validator walks the tree); we only constrain
/// the shape the model can't get wrong (root presence + the `type`
/// enum at the root).
pub fn tool_def() -> ToolDef {
    ToolDef {
        name: TOOL_NAME.to_owned(),
        description: Some(
            "Emit exactly one SDUI page tree that renders the user's request. \
             The tool call is your entire reply; do not also emit prose."
                .to_owned(),
        ),
        input_schema: json!({
            "type": "object",
            "required": ["root"],
            "properties": {
                "root": {
                    "type": "object",
                    "required": ["id", "type"],
                    "properties": {
                        "id":   { "type": "string", "minLength": 1, "maxLength": MAX_ID_LEN },
                        "type": { "type": "string", "enum": KIND_ALLOW },
                        "children": { "type": "array", "maxItems": MAX_WIDTH },
                        "slots":    { "type": "object" }
                    },
                    "additionalProperties": true
                }
            }
        }),
    }
}

// ---------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------

/// Decode + validate a tool-call payload. Returns the validated tree
/// shape `{"root": …}` ready to forward as the `full-render` event.
fn parse_and_validate(args: JsonValue) -> Result<JsonValue, String> {
    // Defensive: some models stringify the entire tool-arguments object
    // (esp. via the CLI runner) so `args` arrives as a JSON-encoded
    // string. Re-parse once before insisting on an object shape.
    let args = coerce_json_string(args);
    let obj = args.as_object().ok_or("top-level args must be object")?;
    let root_raw = obj.get("root").ok_or("missing `root`")?;
    // Same defense one level down: if `root` is a JSON string, decode
    // it. Keeps the validator strict but tolerates a common
    // model-failure mode.
    let mut root = coerce_json_string(root_raw.clone());
    // Best-effort normalisation: map a handful of well-known synonyms
    // the model reaches for (`line` → `chart`, `container` → `stack`,
    // etc.) and drop primitive entries from `children` arrays. Keeps
    // the validator strict (it still rejects unknown kinds) while
    // absorbing the most common LLM slip-ups without a re-prompt.
    normalize_tree(&mut root);
    validate_node(&root, 0)?;
    Ok(json!({ "root": root }))
}

/// Map common LLM kind synonyms to allowed kinds and drop non-object
/// children. Recursive; idempotent.
fn normalize_tree(node: &mut JsonValue) {
    let Some(obj) = node.as_object_mut() else {
        return;
    };
    // 1) Coerce well-known synonyms on `type`.
    if let Some(JsonValue::String(t)) = obj.get_mut("type") {
        let mapped = match t.as_str() {
            // Chart subtypes → chart (preserve the original on `variant`).
            "line" | "bar" | "area" | "pie" | "donut" | "doughnut" | "scatter" => {
                Some(("chart", Some(t.clone())))
            }
            // Layout containers → stack.
            "container" | "div" | "section" | "box" | "wrapper" => Some(("stack", None)),
            // Input synonyms → field.
            "input" | "textarea" | "checkbox" | "radio" | "number" | "email_input" => {
                Some(("field", Some(t.clone())))
            }
            // Typography synonyms → text / heading.
            "label" | "paragraph" | "p" | "span" => Some(("text", None)),
            "title" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some(("heading", None)),
            // Unsupported decorative bits → custom (so the page still renders).
            "image" | "img" | "icon" | "divider" | "spacer" | "hr" | "avatar" => {
                Some(("custom", Some(t.clone())))
            }
            _ => None,
        };
        if let Some((new_kind, variant)) = mapped {
            *t = new_kind.to_owned();
            if let Some(v) = variant {
                obj.entry("variant").or_insert_with(|| JsonValue::String(v));
            }
        }
    }
    // 2) Drop non-object children + recurse.
    if let Some(JsonValue::Array(children)) = obj.get_mut("children") {
        children.retain(|c| c.is_object());
        for child in children.iter_mut() {
            normalize_tree(child);
        }
    }
    // 3) Recurse into slots' nested SDUI nodes too.
    if let Some(JsonValue::Object(slots)) = obj.get_mut("slots") {
        for v in slots.values_mut() {
            if v.is_object() && v.get("type").and_then(|t| t.as_str()).is_some() {
                normalize_tree(v);
            }
        }
    }
}

/// If `v` is a JSON string that parses as a JSON value, return that
/// parsed value; otherwise return `v` unchanged. Idempotent.
fn coerce_json_string(v: JsonValue) -> JsonValue {
    if let JsonValue::String(s) = &v {
        if let Ok(parsed) = serde_json::from_str::<JsonValue>(s) {
            return parsed;
        }
    }
    v
}

fn validate_node(node: &JsonValue, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err(format!("max depth exceeded ({MAX_DEPTH})"));
    }
    let obj = node.as_object().ok_or_else(|| {
        let snippet = first_n_chars(&node.to_string(), 80);
        format!("node must be an object, got: {snippet}")
    })?;
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("missing or non-string `id`")?;
    if id.is_empty() || id.len() > MAX_ID_LEN {
        return Err(format!("id length must be 1..={MAX_ID_LEN}"));
    }
    let kind = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or("missing or non-string `type`")?;
    if !KIND_ALLOW.contains(&kind) {
        return Err(format!("unknown component kind: `{kind}`"));
    }
    if let Some(children) = obj.get("children") {
        let arr = children
            .as_array()
            .ok_or("`children` must be an array when present")?;
        if arr.len() > MAX_WIDTH {
            return Err(format!("max width exceeded ({MAX_WIDTH})"));
        }
        for child in arr {
            validate_node(child, depth + 1)?;
        }
    }
    // Allow optional `slots: object` of arbitrary shape; recurse into
    // any nested SDUI nodes the model may have stuck inside slots.
    if let Some(slots) = obj.get("slots") {
        if !slots.is_object() {
            return Err("`slots` must be an object when present".into());
        }
        for v in slots.as_object().unwrap().values() {
            if v.is_object() && v.get("type").and_then(|t| t.as_str()).is_some() {
                validate_node(v, depth + 1)?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn page(children: JsonValue) -> JsonValue {
        json!({ "id": "r", "type": "page", "children": children })
    }

    #[test]
    fn validates_minimal_tree() {
        let args = json!({ "root": { "id": "r", "type": "page" } });
        let out = parse_and_validate(args).unwrap();
        assert_eq!(out["root"]["type"], "page");
    }

    #[test]
    fn rejects_unknown_kind() {
        let args = json!({ "root": { "id": "r", "type": "no_such" } });
        let err = parse_and_validate(args).unwrap_err();
        assert!(err.contains("unknown component kind"), "got: {err}");
    }

    #[test]
    fn normalises_line_chart_synonym() {
        let args = json!({
            "root": page(json!([
                { "id": "c1", "type": "line", "data": [1, 2, 3] }
            ]))
        });
        let out = parse_and_validate(args).unwrap();
        assert_eq!(out["root"]["children"][0]["type"], "chart");
        assert_eq!(out["root"]["children"][0]["variant"], "line");
    }

    #[test]
    fn normalises_container_to_stack() {
        let args = json!({
            "root": page(json!([
                { "id": "c", "type": "container", "children": [
                    { "id": "t", "type": "label", "value": "hi" }
                ]}
            ]))
        });
        let out = parse_and_validate(args).unwrap();
        assert_eq!(out["root"]["children"][0]["type"], "stack");
        assert_eq!(out["root"]["children"][0]["children"][0]["type"], "text");
    }

    #[test]
    fn drops_primitive_children() {
        let args = json!({
            "root": page(json!([
                "stray string",
                42,
                { "id": "h", "type": "heading", "value": "ok" }
            ]))
        });
        let out = parse_and_validate(args).unwrap();
        let kids = out["root"]["children"].as_array().unwrap();
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0]["type"], "heading");
    }

    #[test]
    fn rejects_missing_id() {
        let args = json!({ "root": { "type": "page" } });
        let err = parse_and_validate(args).unwrap_err();
        assert!(err.contains("missing or non-string `id`"), "got: {err}");
    }

    #[test]
    fn rejects_depth_13() {
        // Build a chain of stacks 13 deep beneath root (depths 1..=13).
        let mut node = json!({ "id": "leaf", "type": "stack" });
        for i in 0..12 {
            node = json!({
                "id": format!("n{i}"),
                "type": "stack",
                "children": [node],
            });
        }
        let args = json!({
            "root": { "id": "r", "type": "page", "children": [node] }
        });
        let err = parse_and_validate(args).unwrap_err();
        assert!(err.contains("max depth"), "got: {err}");
    }

    #[test]
    fn rejects_width_65() {
        let kids: Vec<JsonValue> = (0..65)
            .map(|i| json!({ "id": format!("k{i}"), "type": "text" }))
            .collect();
        let args = json!({ "root": page(JsonValue::Array(kids)) });
        let err = parse_and_validate(args).unwrap_err();
        assert!(err.contains("max width"), "got: {err}");
    }

    #[test]
    fn rejects_non_object_root() {
        let args = json!({ "root": "not-an-object" });
        let err = parse_and_validate(args).unwrap_err();
        assert!(err.contains("node must be an object"), "got: {err}");
    }

    #[test]
    fn coerces_stringified_root() {
        // Model wraps the tree in a JSON string (observed with the
        // haiku CLI runner). Validator should decode + accept.
        let inner =
            r#"{"id":"r","type":"page","children":[{"id":"h","type":"heading","value":"Hi"}]}"#;
        let args = json!({ "root": inner });
        let out = parse_and_validate(args).unwrap();
        assert_eq!(out["root"]["type"], "page");
        assert_eq!(out["root"]["children"][0]["type"], "heading");
    }

    #[test]
    fn coerces_stringified_args() {
        // Whole tool-arguments object stringified.
        let inner = r#"{"root":{"id":"r","type":"page"}}"#;
        let args = JsonValue::String(inner.to_owned());
        let out = parse_and_validate(args).unwrap();
        assert_eq!(out["root"]["type"], "page");
    }

    #[test]
    fn rejects_missing_root() {
        let args = json!({});
        let err = parse_and_validate(args).unwrap_err();
        assert!(err.contains("missing `root`"), "got: {err}");
    }

    #[test]
    fn accepts_nested_dashboard() {
        let args = json!({
            "root": {
                "id": "r", "type": "page",
                "children": [
                    { "id": "ht", "type": "heading", "value": "Title" },
                    {
                        "id": "kg", "type": "grid",
                        "children": [
                            { "id": "k1", "type": "kpi" },
                            { "id": "k2", "type": "kpi" }
                        ]
                    }
                ]
            }
        });
        parse_and_validate(args).unwrap();
    }

    #[test]
    fn extracts_bare_json_object_from_prose() {
        let prose = r#"Here you go: {"root":{"id":"r","type":"page"}} cheers!"#;
        let v = extract_first_json_object(prose).expect("extract");
        assert_eq!(v["root"]["type"], "page");
    }

    #[test]
    fn extracts_json_inside_code_fence() {
        let prose = "```json\n{\n  \"root\": { \"id\": \"r\", \"type\": \"page\" }\n}\n```";
        let v = extract_first_json_object(prose).expect("extract");
        assert_eq!(v["root"]["id"], "r");
    }

    #[test]
    fn extract_handles_braces_inside_strings() {
        let prose = r#"reply: {"root":{"id":"r","type":"text","value":"a {fake} brace"}} done"#;
        let v = extract_first_json_object(prose).expect("extract");
        assert_eq!(v["root"]["value"], "a {fake} brace");
    }

    #[test]
    fn extract_returns_none_when_no_object() {
        assert!(extract_first_json_object("no json here at all").is_none());
    }

    /// Drift gate: every member of `KIND_ALLOW` must appear in the TS
    /// `Kind` union, and vice versa. When this fails, sync both lists
    /// rather than relaxing the test.
    #[test]
    fn kind_allow_matches_ts_union() {
        // CARGO_MANIFEST_DIR = examples/flow-agent
        let ts_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("packages")
            .join("starter-sdui-react")
            .join("src")
            .join("registry")
            .join("types.ts");
        let src = std::fs::read_to_string(&ts_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", ts_path.display()));

        // Slice the `export type Kind = …;` block.
        let start = src
            .find("export type Kind")
            .expect("Kind union not found in types.ts");
        let rest = &src[start..];
        let end = rest.find(';').expect("Kind union has no trailing `;`");
        let block = &rest[..end];

        // Pull every "string-literal" entry.
        let mut ts_kinds: Vec<&str> = Vec::new();
        let mut chars = block.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            if c == '"' {
                let rel = &block[i + 1..];
                if let Some(end_quote) = rel.find('"') {
                    ts_kinds.push(&rel[..end_quote]);
                    // Advance past the closing quote.
                    for _ in 0..end_quote + 1 {
                        chars.next();
                    }
                }
            }
        }
        ts_kinds.sort_unstable();
        ts_kinds.dedup();

        let mut allow: Vec<&str> = KIND_ALLOW.to_vec();
        allow.sort_unstable();
        allow.dedup();

        let missing_in_ts: Vec<&&str> = allow.iter().filter(|k| !ts_kinds.contains(k)).collect();
        let missing_in_allow: Vec<&&str> = ts_kinds.iter().filter(|k| !allow.contains(k)).collect();

        assert!(
            missing_in_ts.is_empty() && missing_in_allow.is_empty(),
            "KIND_ALLOW drift:\n  in KIND_ALLOW but not TS Kind: {missing_in_ts:?}\n  \
             in TS Kind but not KIND_ALLOW: {missing_in_allow:?}\n  \
             types.ts = {}",
            ts_path.display(),
        );
    }

    // ---- BuilderMode + Ask resolver ----

    #[test]
    fn builder_mode_parse_accepts_known_values() {
        assert_eq!(BuilderMode::parse("build"), Some(BuilderMode::Build));
        assert_eq!(BuilderMode::parse("edit"), Some(BuilderMode::Build));
        assert_eq!(BuilderMode::parse("ask"), Some(BuilderMode::Ask));
    }

    #[test]
    fn builder_mode_parse_rejects_unknown() {
        assert_eq!(BuilderMode::parse(""), None);
        assert_eq!(BuilderMode::parse("plan"), None);
        assert_eq!(BuilderMode::parse("BUILD"), None);
    }

    fn ok_result_with(
        text: &str,
    ) -> Result<
        Result<starter_spi::ai::RunResult, starter_spi::ai::RunnerError>,
        tokio::time::error::Elapsed,
    > {
        Ok(Ok(starter_spi::ai::RunResult {
            text: text.to_owned(),
            ..Default::default()
        }))
    }

    #[test]
    fn ask_resolver_returns_streamed_prose() {
        let out = resolve_ask_outcome(ok_result_with(""), "Hello there".to_owned(), None).unwrap();
        assert_eq!(out, "Hello there");
    }

    #[test]
    fn ask_resolver_falls_back_to_result_text() {
        // Some runners populate `result.text` only at completion
        // (no streamed Text events). The resolver should pick it up.
        let out = resolve_ask_outcome(ok_result_with("final answer"), String::new(), None).unwrap();
        assert_eq!(out, "final answer");
    }

    #[test]
    fn ask_resolver_trims_whitespace() {
        let out =
            resolve_ask_outcome(ok_result_with(""), "  spaced reply\n\n".to_owned(), None).unwrap();
        assert_eq!(out, "spaced reply");
    }

    #[test]
    fn ask_resolver_surfaces_empty_reply_as_error() {
        let err = resolve_ask_outcome(ok_result_with(""), String::new(), None).unwrap_err();
        assert!(err.contains("empty reply"), "got: {err}");
    }

    #[test]
    fn ask_resolver_surfaces_runner_error_when_empty() {
        let err = resolve_ask_outcome(ok_result_with(""), String::new(), Some("boom".to_owned()))
            .unwrap_err();
        assert!(err.contains("boom"), "got: {err}");
    }
}
