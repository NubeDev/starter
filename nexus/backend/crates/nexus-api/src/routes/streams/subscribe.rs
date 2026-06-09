//! `GET /api/v1/streams/:id?token=…` — the SSE subscription endpoint.
//!
//! Authenticated by the signed stream token in the query string, never a Bearer
//! (a browser `EventSource` cannot set headers). On a valid token the handler
//! attaches to the running stream for the token's exact (spec + datasource +
//! tenant + permission) key — starting it if none is running — and streams its
//! broadcast events as SSE. The subscription handle lives for the duration of
//! the response; when the client disconnects it drops, decrementing the
//! stream's refcount and tearing it down on the last subscriber.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse as _, Response};
use nexus_engine::stream_registry::{attach, register, Attach};
use nexus_engine::StreamKey;
use serde::Deserialize;
use serde_json::json;
use starter_server::sse;
use tokio_util::sync::CancellationToken;

use crate::middleware::StreamClaims;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SubscribeParams {
    token: String,
}

/// SSE subscription. Returns 401 on a missing/expired/forged token, otherwise an
/// event stream.
#[utoipa::path(
    get,
    path = "/api/v1/streams/{id}",
    tag = "streams",
    operation_id = "subscribe_stream",
    params(
        ("id" = String, Path, description = "Stream id from POST /streams"),
        ("token" = String, Query, description = "Signed subscription token"),
    ),
    responses(
        (status = 200, description = "SSE event stream", content_type = "text/event-stream"),
        (status = 401, description = "Missing/expired/invalid token"),
    ),
)]
pub async fn subscribe_stream(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<SubscribeParams>,
) -> Response {
    let claims = match verify(&state, &params.token, &id) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let subscription = open_subscription(&state, &claims);
    let events = futures::stream::unfold(subscription, |mut sub| async move {
        // Lagged subscribers skip ahead rather than stall the producer; a closed
        // channel ends the SSE stream.
        loop {
            match sub.receiver().recv().await {
                Ok(event) => return Some((event, sub)),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    sse::from_stream(events).into_response()
}

// The error is an axum `Response` (the early-return-a-401 pattern), which is
// larger than the `StreamClaims` success value — that asymmetry is intentional
// here, not a sign the error should be boxed.
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
    // The token is bound to its stream id; a token for one stream can't open
    // another by swapping the path.
    if claims.stream_id != path_id {
        return Err(unauthorized());
    }
    Ok(claims)
}

/// Attach to the stream for `claims`' key, starting a fresh `generate`-driven
/// stream if none is running. The generate source is the M0.5 live demo; real
/// connectors (Kafka/MQTT/Modbus) attach here once restored.
fn open_subscription(state: &AppState, claims: &StreamClaims) -> nexus_engine::Subscription {
    let key = StreamKey {
        spec: format!("generate:{}", claims.stream_id),
        datasource_id: claims.datasource_id.clone(),
        tenant_id: claims.tenant_id.clone(),
        permission: claims.permission.clone(),
    };
    let run_id = claims.stream_id.clone();
    match attach(&key, &run_id) {
        Attach::Existing(sub) => sub,
        Attach::StartNew { run_id } => {
            let token = CancellationToken::new();
            let (input, processors) = demo_source();
            // A spawn failure leaves no stream; the subscriber then sees an empty
            // closed channel and the SSE stream ends cleanly.
            let _ = state.live.spawn(input, processors, &run_id, token.clone());
            register(key, run_id, token)
        }
    }
}

fn demo_source() -> (serde_json::Value, Vec<serde_json::Value>) {
    let input = json!({
        "type": "generate",
        "context": "{ \"value\": 1 }",
        "interval": "1s",
        "batch_size": 1,
    });
    let processors = vec![
        json!({ "type": "json_to_arrow" }),
        json!({ "type": "sql", "query": "SELECT value, now() AS ts FROM flow" }),
    ];
    (input, processors)
}
