//! `POST /api/v1/ingest/{flow_id}` — push JSON into a running `http_ingest` flow.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse as _, Response};
use axum::{Extension, Json};
use nexus_spi::dto::ingest::IngestAccepted;
use serde_json::Value;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::ingest::{self, EnqueueError};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/ingest/{flow_id}",
    tag = "flows",
    operation_id = "push_ingest",
    params(("flow_id" = Uuid, Path, description = "Target http_ingest flow id")),
    request_body = Value,
    responses(
        (status = 200, description = "Accepted onto the flow channel", body = IngestAccepted),
        (status = 404, description = "No such flow accepting pushes in this tenant"),
        (status = 429, description = "Flow channel full; retry after the given seconds"),
    ),
)]
pub async fn push_ingest(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(flow_id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Response {
    let (_caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    match ingest::enqueue(&state, &tenant, flow_id, body).await {
        Ok(accepted) => Json(IngestAccepted { accepted }).into_response(),
        Err(EnqueueError::NotFound) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(EnqueueError::Full { retry_after_secs }) => (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, retry_after_secs.to_string())],
            "flow channel full",
        )
            .into_response(),
        Err(EnqueueError::Store(e)) => IntoResponse(e).into_response(),
    }
}
