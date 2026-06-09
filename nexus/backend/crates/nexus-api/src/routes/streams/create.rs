//! `POST /api/v1/streams` — register a live subscription and mint its SSE token.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::Json;
use nexus_spi::dto::stream::{CreateStreamRequest, CreateStreamResponse};
use starter_server::error::IntoResponse;
use starter_spi::Error;
use uuid::Uuid;

use crate::middleware::StreamClaims;
use crate::state::AppState;

/// Mint a short-lived token scoped to a new stream id and return the URL the
/// browser opens an `EventSource` against. The token — not a Bearer — is what
/// authenticates the subscription.
#[utoipa::path(
    post,
    path = "/api/v1/streams",
    tag = "streams",
    operation_id = "create_stream",
    request_body = CreateStreamRequest,
    responses((status = 200, description = "Stream created", body = CreateStreamResponse)),
)]
pub async fn create_stream(
    State(state): State<AppState>,
    Json(req): Json<CreateStreamRequest>,
) -> Result<Json<CreateStreamResponse>, IntoResponse> {
    let stream_id = Uuid::new_v4();
    let ttl = state.stream_token_ttl.as_secs().max(1);
    let exp = now_secs().map_err(IntoResponse)? + ttl;

    // M0.5 tenant/permission default to the single dev context; M1 binds them
    // from the authenticated principal and datasource ownership.
    let claims = StreamClaims {
        stream_id: stream_id.to_string(),
        datasource_id: req.datasource_id.to_string(),
        tenant_id: "dev".into(),
        permission: "view".into(),
        exp,
    };
    let token = state.stream_signer.mint(&claims);

    Ok(Json(CreateStreamResponse {
        id: stream_id,
        subscribe_url: format!("/api/v1/streams/{stream_id}?token={token}"),
        token,
        expires_in_secs: ttl,
    }))
}

fn now_secs() -> Result<u64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })
}
