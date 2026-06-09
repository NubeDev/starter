//! `POST /api/v1/streams` — authorize a live subscription and mint its SSE token.
//!
//! Bearer-authed (it runs behind the principal layer). It checks the caller may
//! `view` the datasource the panel reads, parks the vetted SQL under the new
//! stream id, and returns a short-lived signed token bound to the caller's real
//! tenant and permission. The browser then opens an `EventSource` against the
//! returned URL carrying only that token — the not-Bearer SSE path.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::stream::{CreateStreamRequest, CreateStreamResponse};
use nexus_store::datasource;
use starter_spi::auth::Principal;
use starter_spi::Error;
use uuid::Uuid;

use super::pending;
use crate::authz::{self, ACTION_VIEW, KIND_DATASOURCE};
use crate::middleware::tenant::caller;
use crate::middleware::StreamClaims;
use crate::state::AppState;

/// How often a live panel re-runs its query. The live SQL path is a poll loop
/// (the engine has no streaming sql input), so this is the panel's refresh
/// cadence — frequent enough to feel live, bounded so one panel can't hammer a
/// datasource.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[utoipa::path(
    post,
    path = "/api/v1/streams",
    tag = "streams",
    operation_id = "create_stream",
    request_body = CreateStreamRequest,
    responses(
        (status = 200, description = "Stream created", body = CreateStreamResponse),
        (status = 403, description = "Not allowed to view the datasource"),
        (status = 404, description = "Datasource not found in this tenant"),
    ),
)]
pub async fn create_stream(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateStreamRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // The datasource must be visible to the tenant (RLS) and the caller must be
    // allowed to view it — the same gate the one-shot query path will use.
    match datasource::get(&state.metadata, &tenant, req.datasource_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (axum::http::StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return starter_server::error::IntoResponse(e).into_response(),
    }
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_DATASOURCE,
        &req.datasource_id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }

    let stream_id = Uuid::new_v4();
    let ttl = Duration::from_secs(state.stream_token_ttl.as_secs().max(1));
    let exp = match now_secs() {
        Ok(n) => n + ttl.as_secs(),
        Err(e) => return starter_server::error::IntoResponse(e).into_response(),
    };

    // Park the vetted SQL for the subscriber; the token carries identity only.
    pending::put(
        &stream_id.to_string(),
        req.sql,
        POLL_INTERVAL,
        ttl,
        Instant::now(),
    );

    let claims = StreamClaims {
        stream_id: stream_id.to_string(),
        datasource_id: req.datasource_id.to_string(),
        tenant_id: tenant,
        permission: ACTION_VIEW.to_string(),
        exp,
    };
    let token = state.stream_signer.mint(&claims);

    Json(CreateStreamResponse {
        id: stream_id,
        subscribe_url: format!("/api/v1/streams/{stream_id}?token={token}"),
        token,
        expires_in_secs: ttl.as_secs(),
    })
    .into_response()
}

fn now_secs() -> Result<u64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })
}
