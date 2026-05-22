//! `GET /v1/agent-log` — paged AI-agent projection.

use std::sync::Arc;

use axum::extract::{Extension, Query};
use axum::routing::get;
use axum::{Json, Router};
use starter_changelog::{ChangeFilter, ChangePage};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::Error;

use crate::AgentLogService;

/// Build the agent-log router.
pub fn agent_log_router<S>(service: Arc<AgentLogService>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<S>::new()
        .route("/v1/agent-log", get(list))
        .layer(Extension(service))
}

async fn list(
    Extension(service): Extension<Arc<AgentLogService>>,
    Query(filter): Query<ChangeFilter>,
    req: axum::extract::Request,
) -> Result<Json<ChangePage>, IntoResponse> {
    let principal = req
        .extensions()
        .get::<Principal>()
        .cloned()
        .ok_or(IntoResponse(Error::Unauthenticated))?;
    let page = service
        .list(&principal, filter)
        .await
        .map_err(IntoResponse)?;
    Ok(Json(page))
}
