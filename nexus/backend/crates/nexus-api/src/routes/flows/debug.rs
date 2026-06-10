//! Flow debug & values endpoints.
//!
//! `POST /api/v1/flows/{id}/debug/enable` turns on per-node value/sample capture
//! for a running flow and mints a short-lived signed token plus the SSE URL to
//! open. `POST .../debug/disable` turns capture back off. `GET .../debug/stream`
//! is the SSE subscription, authed by the token in the query string because a
//! browser `EventSource` cannot send a Bearer header (the same not-Bearer path as
//! live query streams).
//!
//! Capture defaults to off when a flow starts: the per-node taps are always
//! installed, but they only sample and publish while enabled, so an undebugged
//! flow pays almost nothing. Enabling never restarts the flow.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse as _, Response};
use axum::{Extension, Json};
use nexus_engine::flow::debug as flow_debug;
use nexus_spi::dto::flow::{FlowDebugEnableResponse, FlowDebugEvent, FlowDebugStatus};
use serde::Deserialize;
use starter_server::sse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_EDIT, KIND_FLOW};
use crate::middleware::tenant::caller;
use crate::middleware::StreamClaims;
use crate::state::AppState;

/// Permission marker carried in the debug stream token. Distinct from a query
/// `view` so a debug token cannot be replayed against the live-query routes.
const DEBUG_PERMISSION: &str = "flow_debug";

#[utoipa::path(
    post,
    path = "/api/v1/flows/{id}/debug/enable",
    tag = "flows",
    operation_id = "enable_flow_debug",
    params(("id" = Uuid, Path, description = "Flow id")),
    responses(
        (status = 200, description = "Debug enabled", body = FlowDebugEnableResponse),
        (status = 403, description = "Not allowed to debug this flow"),
        (status = 404, description = "Flow not found or not running"),
    ),
)]
pub async fn enable_flow_debug(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = require_edit(&state, caller, &id, &tenant).await {
        return resp;
    }

    let channel = match flow_debug::lookup(&id.to_string()) {
        Some(c) => c,
        None => return (StatusCode::NOT_FOUND, "flow not running").into_response(),
    };
    channel.set_enabled(true);

    let ttl = Duration::from_secs(state.stream_token_ttl.as_secs().max(1));
    let exp = match now_secs() {
        Ok(n) => n + ttl.as_secs(),
        Err(resp) => return resp,
    };
    let claims = StreamClaims {
        stream_id: id.to_string(),
        datasource_id: String::new(),
        tenant_id: tenant,
        permission: DEBUG_PERMISSION.to_string(),
        exp,
    };
    let token = state.stream_signer.mint(&claims);

    Json(FlowDebugEnableResponse {
        status: FlowDebugStatus {
            enabled: true,
            node_count: channel.node_count(),
        },
        stream_url: format!("/api/v1/flows/{id}/debug/stream?token={token}"),
        token,
        expires_in_secs: ttl.as_secs(),
    })
    .into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/flows/{id}/debug/disable",
    tag = "flows",
    operation_id = "disable_flow_debug",
    params(("id" = Uuid, Path, description = "Flow id")),
    responses(
        (status = 200, description = "Debug disabled", body = FlowDebugStatus),
        (status = 403, description = "Not allowed to debug this flow"),
        (status = 404, description = "Flow not found or not running"),
    ),
)]
pub async fn disable_flow_debug(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = require_edit(&state, caller, &id, &tenant).await {
        return resp;
    }
    match flow_debug::lookup(&id.to_string()) {
        Some(channel) => {
            channel.set_enabled(false);
            Json(FlowDebugStatus {
                enabled: false,
                node_count: channel.node_count(),
            })
            .into_response()
        }
        None => (StatusCode::NOT_FOUND, "flow not running").into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct StreamParams {
    token: String,
}

/// SSE subscription to a running flow's debug events. Authed by the signed token
/// minted on enable, never a Bearer (a browser `EventSource` cannot set headers).
#[utoipa::path(
    get,
    path = "/api/v1/flows/{id}/debug/stream",
    tag = "flows",
    operation_id = "stream_flow_debug",
    params(
        ("id" = Uuid, Path, description = "Flow id"),
        ("token" = String, Query, description = "Signed debug token from enable"),
    ),
    responses(
        (status = 200, description = "SSE event stream", content_type = "text/event-stream"),
        (status = 401, description = "Missing/expired/invalid token"),
        (status = 404, description = "Flow not running"),
    ),
)]
pub async fn stream_flow_debug(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<StreamParams>,
) -> Response {
    // Verify the token: valid signature, unexpired, bound to this flow id, and
    // minted for debug (not a replayed live-query token).
    let now = match now_secs() {
        Ok(n) => n,
        Err(resp) => return resp,
    };
    let claims = match state.stream_signer.verify(&params.token, now) {
        Ok(c) => c,
        Err(_) => return (StatusCode::UNAUTHORIZED, "unauthorized").into_response(),
    };
    if claims.stream_id != id.to_string() || claims.permission != DEBUG_PERMISSION {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    let channel = match flow_debug::lookup(&id.to_string()) {
        Some(c) => c,
        None => return (StatusCode::NOT_FOUND, "flow not running").into_response(),
    };
    let receiver = channel.subscribe();
    let events = futures::stream::unfold(receiver, |mut rx| async move {
        // A lagging subscriber skips ahead rather than stalling the producer; a
        // closed channel (run ended) ends the SSE stream.
        loop {
            match rx.recv().await {
                Ok(event) => return Some((event, rx)),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    sse::from_stream::<_, FlowDebugEvent>(events).into_response()
}

/// Require the caller's `edit` grant on the flow (debug exposes row values, so it
/// gates on edit, not just view). The error is an axum `Response` (the
/// early-return-a-403 pattern), larger than the `()` success — that asymmetry is
/// intentional here, matching the streams subscribe handler.
#[allow(clippy::result_large_err)]
async fn require_edit(
    state: &AppState,
    caller: &Principal,
    id: &Uuid,
    tenant: &str,
) -> Result<(), Response> {
    authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_FLOW,
        &id.to_string(),
        tenant,
    )
    .await
}

#[allow(clippy::result_large_err)]
fn now_secs() -> Result<u64, Response> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "clock error").into_response())
}
