//! `GET /v1/audit` — paged user-audit projection.
//!
//! Thin transport (SCOPE R3): pulls `Principal` from the request
//! extension (`with_principal` middleware installed it), forwards
//! the query string as a [`ChangeFilter`], and returns whatever
//! [`AuditService::list`] yields after the visibility gate.
//!
//! OpenAPI registration is deferred — adding `utoipa::ToSchema` to
//! `Change` requires touching `starter-spi`. TODO: wire `AuditApi`
//! once spi schemas land.

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

// Quieten the unused-import lint when `Response` is needed only by
// `IntoResponse`'s axum impl — kept explicit so the file documents
// the response shape at a glance.
#[allow(dead_code)]
fn _response_marker(_: Response) {}
