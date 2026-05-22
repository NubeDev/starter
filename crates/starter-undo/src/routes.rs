//! `POST /v1/undo` and `POST /v1/redo`.
//!
//! Targets the authenticated principal — no body field overrides
//! the actor (SCOPE §"Non-goals" — no global undo across actors).
//! The response carries the `group_id` that was undone / redone so
//! the UI can refresh the affected resources.

use std::sync::Arc;

use axum::extract::Extension;
use axum::routing::post;
use axum::{Json, Router};
use serde::Serialize;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::changelog::Actor;
use starter_spi::Error;
use utoipa::ToSchema;

use crate::UndoService;

/// Build the undo / redo router.
pub fn undo_router<S>(service: Arc<UndoService>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<S>::new()
        .route("/v1/undo", post(undo))
        .route("/v1/redo", post(redo))
        .layer(Extension(service))
}

/// Response body of `/v1/undo` and `/v1/redo`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UndoResponse {
    /// The `group_id` that was applied.
    pub group_id: String,
}

/// `POST /v1/undo` — undo the most recent group for the
/// authenticated principal.
#[utoipa::path(
    post,
    path = "/v1/undo",
    tag = "undo",
    responses(
        (status = 200, description = "Group that was undone", body = UndoResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "No undoable group for this actor"),
        (status = 409, description = "Stale `resource_version` — refuse"),
    ),
)]
async fn undo(
    Extension(service): Extension<Arc<UndoService>>,
    req: axum::extract::Request,
) -> Result<Json<UndoResponse>, IntoResponse> {
    let actor = actor_from_request(&req)?;
    let group = service.undo(&actor).await.map_err(IntoResponse)?;
    Ok(Json(UndoResponse { group_id: group.0 }))
}

/// `POST /v1/redo` — redo the most recently undone group.
#[utoipa::path(
    post,
    path = "/v1/redo",
    tag = "undo",
    responses(
        (status = 200, description = "Group that was redone", body = UndoResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "Redo stack empty"),
        (status = 409, description = "Stale `resource_version` — refuse"),
    ),
)]
async fn redo(
    Extension(service): Extension<Arc<UndoService>>,
    req: axum::extract::Request,
) -> Result<Json<UndoResponse>, IntoResponse> {
    let actor = actor_from_request(&req)?;
    let group = service.redo(&actor).await.map_err(IntoResponse)?;
    Ok(Json(UndoResponse { group_id: group.0 }))
}

/// OpenAPI document fragment for the undo / redo router.
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(undo, redo),
    components(schemas(UndoResponse)),
    tags((name = "undo", description = "Per-actor undo / redo"))
)]
pub struct UndoApi;

fn actor_from_request(req: &axum::extract::Request) -> Result<Actor, IntoResponse> {
    let principal = req
        .extensions()
        .get::<Principal>()
        .ok_or(IntoResponse(Error::Unauthenticated))?;
    Ok(Actor::User {
        subject: principal.subject.clone(),
    })
}
