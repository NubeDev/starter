//! `POST /api/v1/admin/registry/tools/{id}/invoke/stream` —
//! SSE-streaming admin tool dispatch.
//!
//! LAYER: transport (REST). Same body shape, same role + scope
//! gates, and same `CallerIdentity` scoping as the synchronous
//! sibling [`super::invoke`]; the only difference is the wire:
//! frames flow as SSE [`StreamFrame`] events instead of a single
//! JSON response.
//!
//! ## Frames emitted
//!
//! 1. `connected { model: null }` — opens the stream so a client
//!    observes the connection before the runner starts (mirrors
//!    chat).
//! 2. Today: nothing in the middle. Every existing tool implements
//!    `Tool::invoke` synchronously, so the handler awaits the
//!    sync result and translates it. A future
//!    `Tool::invoke_stream` (or sidecar `StreamingTool`) trait
//!    will let long-running tools emit `text` / `result` /
//!    progress frames here; the wire shape is stable.
//! 3. Terminal frame:
//!    - on `Ok(value)` → `result { value }` then
//!      `done { status: "ok", latency_ms }`,
//!    - on `Err(e)` → `error { message: e.to_string() }` then
//!      `done { status, latency_ms }`.
//!
//! See [docs/design/admin/](../../../../docs/design/admin/README.md)
//! §"Streaming invoke".

use std::convert::Infallible;
use std::time::Instant;

use axum::extract::{Path, State};
use axum::http::{Method, StatusCode};
use axum::response::sse::{KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Extension, Json};
use futures::stream::{self, Stream};
use serde::Deserialize;
use serde_json::{json, Value};
use starter_ext_spi::identity::CallerIdentity;
use starter_spi::auth::Principal;
use starter_spi::error::Error;
use tracing::info;

use crate::admin::AdminState;
use crate::routes::stream_frames::{frame_to_sse, StreamFrame};
use crate::routes::{RouteMeta, RouteRegistrar};

/// Build the streaming invoke registrar.
pub fn admin_invoke_stream_registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::POST,
        "/api/v1/admin/registry/tools/{tool_id}/invoke/stream",
        post(invoke_stream).with_state(state),
        RouteMeta::new()
            .describe(
                "SSE-stream a tool dispatch in the shared StreamFrame shape (chat-compatible).",
            )
            .tag("admin")
            .request_schema(json!({
                "type": "object",
                "required": ["tenant"],
                "properties": {
                    "tenant": { "type": "string", "minLength": 1 },
                    "input":  { "description": "Tool-specific input." }
                }
            })),
    )
}

#[derive(Debug, Deserialize)]
struct InvokeBody {
    #[serde(default)]
    tenant: Option<String>,
    #[serde(default)]
    input: Value,
}

async fn invoke_stream(
    State(state): State<AdminState>,
    Path(tool_id): Path<String>,
    principal: Option<Extension<Principal>>,
    Json(body): Json<InvokeBody>,
) -> Response {
    let tenant = body.tenant.unwrap_or_default().trim().to_owned();
    if tenant.is_empty() {
        return bad_request("`tenant` is required and must be a non-empty string");
    }
    let Some(tool) = state.tools.get(&tool_id).cloned() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "unknown tool", "tool_id": tool_id})),
        )
            .into_response();
    };
    let actor_subject = principal
        .as_ref()
        .map(|Extension(p)| p.subject.clone())
        .unwrap_or_else(|| "anonymous".to_owned());
    let caller = CallerIdentity {
        tenant_id: Some(tenant.clone()),
        user_id: Some(actor_subject.clone()),
        roles: vec!["admin".to_owned()],
        request_id: String::new(),
    };
    let actor = starter_spi::changelog::Actor::User {
        subject: actor_subject.clone(),
    };
    let started = Instant::now();
    let result = starter_ext_supervisor::caller_local::scope(
        caller,
        starter_undo::actor_local::scope(actor, tool.invoke(body.input)),
    )
    .await;
    let latency_ms = started.elapsed().as_millis() as u64;
    let status = audit(&actor_subject, &tenant, &tool_id, &result, latency_ms);
    let frames = build_frames(result, status, latency_ms);
    Sse::new(frames)
        .keep_alive(KeepAlive::new())
        .into_response()
}

/// Convert the terminal `Result` into the in-order frame sequence
/// the client decoder expects.
fn build_frames(
    result: Result<Value, Error>,
    status: &'static str,
    latency_ms: u64,
) -> impl Stream<Item = Result<axum::response::sse::Event, Infallible>> + Send + 'static {
    let mut frames: Vec<StreamFrame> = Vec::with_capacity(3);
    frames.push(StreamFrame::Connected { model: None });
    match result {
        Ok(value) => {
            frames.push(StreamFrame::Result { value });
            frames.push(StreamFrame::done_invoke(status, latency_ms));
        }
        Err(e) => {
            frames.push(StreamFrame::Error {
                message: e.to_string(),
            });
            frames.push(StreamFrame::done_invoke(status, latency_ms));
        }
    }
    stream::iter(frames.into_iter().map(|f| frame_to_sse(&f)))
}

/// Returns the status label the audit line emits — also reused
/// in the terminal `done` frame so wire consumers can switch on
/// it without parsing the error message.
fn audit(
    actor: &str,
    tenant: &str,
    tool_id: &str,
    result: &Result<Value, Error>,
    latency_ms: u64,
) -> &'static str {
    let status: &'static str = match result {
        Ok(_) => "ok",
        Err(e) => match e {
            Error::NotFound { .. } => "not_found",
            Error::Invalid { .. } => "invalid",
            Error::Unauthenticated => "unauthenticated",
            Error::Forbidden => "forbidden",
            Error::Conflict { .. } => "conflict",
            Error::Internal { .. } => "internal",
            _ => "other",
        },
    };
    info!(
        target: "rubix.admin.invoke",
        actor = %actor,
        tenant = %tenant,
        tool_id = %tool_id,
        status = %status,
        latency_ms = latency_ms,
        stream = true,
        "admin tool invoke (stream)",
    );
    status
}

fn bad_request(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "bad_request", "message": message})),
    )
        .into_response()
}
