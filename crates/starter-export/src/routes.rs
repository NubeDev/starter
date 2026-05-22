//! `POST /v1/export` HTTP router.
//!
//! Per SCOPE.md R3 the handler is intentionally thin: extract →
//! dispatch through the [`Exporter`] trait → shape the response. The
//! consumer registers whichever concrete [`Exporter`] they want at
//! mount time; this crate ships no opinions about which format is
//! "the" default.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::Extension;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};

use crate::exporter::{ExportRequest, Exporter};

/// Shared state for the export router.
#[derive(Clone)]
pub struct ExportRoutesState {
    /// The active [`Exporter`]. Often a small dispatcher that fans
    /// out to per-format backends.
    pub exporter: Arc<dyn Exporter>,
}

/// Build the export router. Mount under the API root the consumer
/// uses for the rest of its v1 surface.
pub fn export_router<S>(state: ExportRoutesState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<S>::new()
        .route("/v1/export", post(export))
        .layer(Extension(state))
}

async fn export(
    Extension(state): Extension<ExportRoutesState>,
    Json(request): Json<ExportRequest>,
) -> Result<Response, starter_server::error::IntoResponse> {
    let result = state
        .exporter
        .export(request)
        .await
        .map_err(|e| starter_server::error::IntoResponse(starter_spi::Error::from(e)))?;

    let disposition = format!("attachment; filename=\"{}\"", result.full_filename());
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, result.format.content_type())
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(Body::from(result.bytes))
        .map_err(|e| {
            starter_server::error::IntoResponse(starter_spi::Error::Internal {
                source: Box::new(e),
            })
        })
}
