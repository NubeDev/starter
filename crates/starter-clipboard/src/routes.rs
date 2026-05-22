//! `POST /v1/clipboard/copy` and `POST /v1/clipboard/paste`.
//!
//! Copy persists the `after` snapshot keyed by `(principal, kind)`;
//! paste dispatches through the [`ReversibleRegistry`] inside a
//! recorder transaction so the new rows undo as one group.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use axum::extract::Extension;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::ChangeRecorder;
use starter_spi::Error;
use starter_undo::ReversibleRegistry;
use utoipa::ToSchema;

use crate::ClipboardService;

/// Shared state for the clipboard router.
#[derive(Clone)]
pub struct ClipboardRoutesState {
    /// Underlying clipboard service.
    pub service: Arc<ClipboardService>,
    /// Registry of per-kind reversible impls (paste dispatch).
    pub registry: Arc<ReversibleRegistry>,
    /// Recorder used to open the paste transaction.
    pub recorder: Arc<dyn ChangeRecorder>,
}

/// Build the clipboard router.
pub fn clipboard_router<S>(state: ClipboardRoutesState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<S>::new()
        .route("/v1/clipboard/copy", post(copy))
        .route("/v1/clipboard/paste", post(paste))
        .layer(Extension(state))
}

/// `POST /v1/clipboard/copy` body.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CopyRequest {
    /// `ResourceRef::kind` of the source.
    pub kind: String,
    /// `after` snapshot of the source resource. Opaque to starter.
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
}

/// `POST /v1/clipboard/copy` response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CopyResponse {
    /// Server-assigned clipboard entry id.
    pub id: String,
}

/// `POST /v1/clipboard/paste` body.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PasteRequest {
    /// Clipboard entry to paste.
    pub entry_id: String,
    /// Fields to override on top of the clipboard payload (merged
    /// shallow; override keys win).
    #[serde(default)]
    #[schema(value_type = Object)]
    pub overrides: serde_json::Value,
}

/// `POST /v1/clipboard/paste` response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PasteResponse {
    /// Newly created resources.
    pub created: Vec<ResourceRef>,
}

/// `POST /v1/clipboard/copy` — persist a resource snapshot to
/// the principal-scoped clipboard.
#[utoipa::path(
    post,
    path = "/v1/clipboard/copy",
    tag = "clipboard",
    request_body = CopyRequest,
    responses(
        (status = 200, description = "Clipboard entry id", body = CopyResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 400, description = "Invalid request body"),
    ),
)]
async fn copy(
    Extension(state): Extension<ClipboardRoutesState>,
    req: axum::extract::Request,
) -> Result<Json<CopyResponse>, IntoResponse> {
    let (parts, body) = req.into_parts();
    let principal = parts
        .extensions
        .get::<Principal>()
        .cloned()
        .ok_or(IntoResponse(Error::Unauthenticated))?;
    let Json(body): Json<CopyRequest> =
        Json::from_request(axum::extract::Request::from_parts(parts, body), &())
            .await
            .map_err(|e| {
                IntoResponse(Error::Invalid {
                    message: format!("invalid request body: {e}"),
                })
            })?;

    let id = state
        .service
        .copy(&principal, body.kind, body.payload)
        .await
        .map_err(IntoResponse)?;
    Ok(Json(CopyResponse { id }))
}

/// `POST /v1/clipboard/paste` — paste a clipboard entry via the
/// per-kind [`starter_undo::ReversibleRegistry`].
#[utoipa::path(
    post,
    path = "/v1/clipboard/paste",
    tag = "clipboard",
    request_body = PasteRequest,
    responses(
        (status = 200, description = "Newly-created resources", body = PasteResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "Clipboard entry not found / expired"),
        (status = 400, description = "Unknown resource kind or invalid body"),
    ),
)]
async fn paste(
    Extension(state): Extension<ClipboardRoutesState>,
    req: axum::extract::Request,
) -> Result<Json<PasteResponse>, IntoResponse> {
    let (parts, body) = req.into_parts();
    let principal = parts
        .extensions
        .get::<Principal>()
        .cloned()
        .ok_or(IntoResponse(Error::Unauthenticated))?;
    let Json(body): Json<PasteRequest> =
        Json::from_request(axum::extract::Request::from_parts(parts, body), &())
            .await
            .map_err(|e| {
                IntoResponse(Error::Invalid {
                    message: format!("invalid request body: {e}"),
                })
            })?;

    // Look up the entry once up front so we can route to the right
    // Reversible impl before opening a transaction.
    let entry = state
        .service
        .store_get(&principal.subject, &body.entry_id)
        .await
        .map_err(IntoResponse)?
        .ok_or(IntoResponse(Error::NotFound {
            what: format!("clipboard entry {}", body.entry_id),
        }))?;

    let reversible = state
        .registry
        .get(&entry.resource_kind)
        .cloned()
        .ok_or(IntoResponse(Error::Invalid {
            message: format!(
                "no Reversible registered for kind {:?}",
                entry.resource_kind
            ),
        }))?;

    // The recorder's `transaction` is `-> Result<()>`; the
    // `Vec<ResourceRef>` from `clone_with` escapes through a
    // shared mutex so we can return it from the handler.
    let created: Arc<StdMutex<Vec<ResourceRef>>> = Arc::new(StdMutex::new(Vec::new()));
    let created_inner = created.clone();
    let svc = state.service.clone();
    let overrides = body.overrides;
    let principal_inner = principal.clone();

    state
        .recorder
        .transaction(Box::new(move |tx| {
            let svc = svc.clone();
            let principal = principal_inner.clone();
            let reversible = reversible.clone();
            let overrides = overrides.clone();
            let entry_id = body.entry_id.clone();
            let created = created_inner.clone();
            Box::pin(async move {
                let refs = svc
                    .paste(&principal, reversible.as_ref(), tx, &entry_id, overrides)
                    .await?;
                *created.lock().unwrap() = refs;
                Ok(())
            })
        }))
        .await
        .map_err(IntoResponse)?;

    let created = std::mem::take(&mut *created.lock().unwrap());
    Ok(Json(PasteResponse { created }))
}

// `Json::from_request` needs `FromRequest` in scope.
use axum::extract::FromRequest;

/// OpenAPI document fragment for the clipboard router.
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(copy, paste),
    components(schemas(
        CopyRequest,
        CopyResponse,
        PasteRequest,
        PasteResponse,
        starter_spi::authz::ResourceRef,
    )),
    tags((name = "clipboard", description = "Server-side copy / paste / duplicate"))
)]
pub struct ClipboardApi;
