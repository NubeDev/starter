//! Alert-event history handler.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::alert::AlertEvent;
use nexus_store::alert::event;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::event_to_dto;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

/// The most recent transitions for the tenant, newest first. Capped server-side.
const EVENT_LIMIT: i64 = 200;

#[utoipa::path(get, path = "/api/v1/alerts/events", tag = "alerts", operation_id = "list_alert_events",
    responses((status = 200, description = "Events", body = [AlertEvent])))]
pub async fn list_events(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match event::list(&state.metadata, &tenant, EVENT_LIMIT).await {
        Ok(es) => Json(es.iter().map(event_to_dto).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
