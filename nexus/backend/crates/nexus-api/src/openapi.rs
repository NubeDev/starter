//! Assemble the full OpenAPI document: nexus-api's route paths merged with the
//! DTO schema surface published by `nexus-spi`.
//!
//! Paths are registered here as handlers land (each `#[utoipa::path]` handler is
//! added to [`Paths`]). The schema components come from `nexus_spi`, so the
//! type contract has a single source of truth that the frontend codegens from.

use utoipa::openapi::{InfoBuilder, OpenApi as OpenApiDoc};
use utoipa::OpenApi;

/// Route paths declared by nexus-api handlers. Empty until the first
/// `#[utoipa::path]` handler is merged in (M0 adds `/query`).
#[derive(OpenApi)]
#[openapi(info(title = "Nexus API", version = "0.1.0"))]
pub struct Paths;

/// The published document: nexus-api paths plus nexus-spi schemas.
pub fn document() -> OpenApiDoc {
    let mut doc = Paths::openapi();
    doc.info = InfoBuilder::new()
        .title("Nexus API")
        .version(env!("CARGO_PKG_VERSION"))
        .description(Some(
            "Control plane for the Nexus observability/BI platform: datasources, \
             one-shot queries, live SSE streams, dashboards, and panels.",
        ))
        .build();
    doc.merge(nexus_spi::openapi::Schemas::openapi());
    doc
}

/// Newtype so the generator and server share one `OpenApi`-producing entry
/// point. `ApiDoc::openapi()` returns the fully merged document.
pub struct ApiDoc;

impl OpenApi for ApiDoc {
    fn openapi() -> OpenApiDoc {
        document()
    }
}
