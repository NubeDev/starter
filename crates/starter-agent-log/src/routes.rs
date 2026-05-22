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

/// `GET /v1/agent-log` — paged list of agent-authored changes.
#[utoipa::path(
    get,
    path = "/v1/agent-log",
    tag = "agent-log",
    params(ChangeFilter),
    responses(
        (status = 200, description = "Page of agent-authored changes", body = ChangePage),
        (status = 401, description = "Unauthenticated"),
    ),
)]
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

/// OpenAPI document fragment for the agent-log router.
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(list),
    components(schemas(
        ChangeFilter,
        ChangePage,
        starter_spi::changelog::Change,
        starter_spi::changelog::ChangeId,
        starter_spi::changelog::GroupId,
        starter_spi::changelog::TraceId,
        starter_spi::changelog::Actor,
        starter_spi::changelog::Op,
        starter_spi::authz::ResourceRef,
    )),
    tags((name = "agent-log", description = "AI-agent activity log"))
)]
pub struct AgentLogApi;
