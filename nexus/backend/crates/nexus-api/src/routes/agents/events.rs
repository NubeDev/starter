//! `GET /api/v1/agents/sessions/:id/events?token=…` — the SSE feed for a running
//! session.
//!
//! Authenticated by the signed session token in the query string, never a Bearer
//! (a browser `EventSource` cannot set headers), exactly like the live-stream
//! subscribe endpoint. On a valid token the handler subscribes to the session's
//! in-memory broadcast and streams its [`nexus_ai::Event`]s as SSE. When the run
//! finishes the channel closes and the SSE stream ends; the durable transcript is
//! then available from `GET /agents/sessions/{id}`.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse as _, Response};
use serde::Deserialize;
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
        (status = 410, description = "Session run already finished"),
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

    let receiver = match state.sessions.subscribe(session_id) {
        Some(rx) => rx,
        // Token valid but the run already completed — no live channel. The
        // transcript is durable, so the client should GET the session instead.
        None => {
            return (
                axum::http::StatusCode::GONE,
                "session run already finished; fetch the session for its transcript",
            )
                .into_response()
        }
    };

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
    sse::from_stream(events).into_response()
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
