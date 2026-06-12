//! `GET /api/v1/agents/sessions/:id/events?token=…` — the SSE feed for a session.
//!
//! Authenticated by the signed session token in the query string, never a Bearer
//! (a browser `EventSource` cannot set headers), exactly like the live-stream
//! subscribe endpoint. On a valid token the handler subscribes to the session's
//! in-memory broadcast and streams its [`nexus_ai::Event`]s as SSE.
//!
//! Live-vs-finished: a session run is fast and its broadcast channel is dropped
//! the instant the run ends, so a client that connects a moment late would find
//! no live channel. Rather than 410 (which an `EventSource` would just retry,
//! looping), the handler falls back to the **persisted** session: it replays the
//! durable transcript as a terminal `Done` event (or an error event for a failed
//! run) and closes. This makes both the live and the already-finished cases serve
//! the answer over the same SSE contract, with no race.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse as _, Response};
use futures::StreamExt as _;
use serde::Deserialize;
use serde_json::{json, Value};
use starter_server::sse;
use uuid::Uuid;

use crate::middleware::StreamClaims;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct EventsParams {
    token: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/sessions/{id}/events",
    tag = "agents",
    operation_id = "subscribe_agent_session",
    params(
        ("id" = String, Path, description = "Session id"),
        ("token" = String, Query, description = "Signed session token from create_agent_session"),
    ),
    responses(
        (status = 200, description = "SSE event stream", content_type = "text/event-stream"),
        (status = 401, description = "Missing/expired/invalid token"),
        (status = 404, description = "Session not found in this tenant"),
    ),
)]
pub async fn subscribe_agent_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<EventsParams>,
) -> Response {
    let claims = match verify(&state, &params.token, &id) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let session_id = match Uuid::parse_str(&claims.stream_id) {
        Ok(u) => u,
        Err(_) => return (axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response(),
    };

    // Live path: a running broadcast channel exists — stream its events as they
    // arrive, terminating when the producer drops the channel.
    if let Some(receiver) = state.sessions.subscribe(session_id) {
        let events = futures::stream::unfold(receiver, |mut rx| async move {
            loop {
                match rx.recv().await {
                    Ok(event) => return Some((event, rx)),
                    // Lagged subscribers skip ahead rather than stall the producer.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                }
            }
        });
        return sse::from_stream(events).into_response();
    }

    // Finished/late path: no live channel. Replay the persisted session as a
    // single terminal event so a client that connected after the (fast) run
    // still gets the answer — no 410, no EventSource reconnect loop.
    let session =
        match nexus_store::agent::get_session(&state.metadata, &claims.tenant_id, session_id).await
        {
            Ok(Some(s)) => s,
            Ok(None) => {
                return (axum::http::StatusCode::NOT_FOUND, "session not found").into_response()
            }
            Err(e) => {
                return starter_server::error::IntoResponse(e).into_response();
            }
        };

    let event = replay_event(&session.status, &session.transcript);
    sse::from_stream(futures::stream::once(async move { event }).boxed()).into_response()
}

/// Build the single terminal SSE event for a finished session from its persisted
/// status + transcript. A `failed`/`cancelled` run yields an error event; any
/// other status yields a `done` event carrying the assistant turn's text. The
/// shape matches the unified `nexus_ai::Event` tags the client already handles.
fn replay_event(status: &str, transcript: &Value) -> Value {
    if status == "failed" || status == "cancelled" {
        return json!({
            "kind": "raw",
            "error": format!("the agent run {status}"),
        });
    }
    let text = assistant_text(transcript).unwrap_or_default();
    json!({ "kind": "done", "text": text })
}

/// Pull the assistant turn's content from a persisted `[{role,content},…]`
/// transcript, or `None` if absent.
fn assistant_text(transcript: &Value) -> Option<String> {
    transcript
        .as_array()?
        .iter()
        .rev()
        .find(|m| m.get("role").and_then(Value::as_str) == Some("assistant"))
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

// The error is an axum `Response` (the early-return-a-401 pattern), larger than
// the success `StreamClaims`; the asymmetry is intentional, as in the streams
// subscribe handler.
#[allow(clippy::result_large_err)]
fn verify(state: &AppState, token: &str, path_id: &str) -> Result<StreamClaims, Response> {
    let unauthorized = || (axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| unauthorized())?;
    let claims = state
        .stream_signer
        .verify(token, now)
        .map_err(|_| unauthorized())?;
    // The token is bound to its session id; a token for one session can't open
    // another by swapping the path.
    if claims.stream_id != path_id {
        return Err(unauthorized());
    }
    Ok(claims)
}
