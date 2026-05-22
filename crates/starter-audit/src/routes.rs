//! `GET /v1/audit` — paged user-audit projection.
//!
//! Thin transport (SCOPE R3): pulls `Principal` from the request
//! extension (`with_principal` middleware installed it), forwards
//! the query string as a [`ChangeFilter`], and returns whatever
//! [`AuditService::list`] yields after the visibility gate.

use std::sync::Arc;

use axum::extract::{Extension, Query};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use starter_changelog::{ChangeFilter, ChangePage};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::Error;

use crate::AuditService;

/// Build the audit router. Generic over the consumer's app state.
pub fn audit_router<S>(service: Arc<AuditService>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<S>::new()
        .route("/v1/audit", get(list))
        .layer(Extension(service))
}

/// `GET /v1/audit` — paged list of user-authored changes.
#[utoipa::path(
    get,
    path = "/v1/audit",
    tag = "audit",
    params(ChangeFilter),
    responses(
        (status = 200, description = "Page of user-authored changes", body = ChangePage),
        (status = 401, description = "Unauthenticated"),
    ),
)]
async fn list(
    Extension(service): Extension<Arc<AuditService>>,
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

/// OpenAPI document fragment for the audit router. Consumers merge
/// this into their own `OpenApi` so `/v1/audit` shows up in
/// `/openapi.json` and the generated client.
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
    tags((name = "audit", description = "User audit log"))
)]
pub struct AuditApi;

// Quieten the unused-import lint when `Response` is needed only by
// `IntoResponse`'s axum impl — kept explicit so the file documents
// the response shape at a glance.
#[allow(dead_code)]
fn _response_marker(_: Response) {}
