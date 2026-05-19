//! `GET /openapi.json`. Serves the OpenAPI document the consumer
//! passed to `ServerBuilder::with_openapi`. The `pnpm codegen` step
//! reads this URL to produce `@nube/starter-client-ts`.

use axum::{routing::get, Json, Router};
use utoipa::openapi::OpenApi;

/// Build the openapi-document router. The doc is captured at build
/// time and served by value — there is no rebuild on request.
pub fn openapi_router<S: Clone + Send + Sync + 'static>(doc: OpenApi) -> Router<S> {
    Router::new().route("/openapi.json", get(move || serve(doc.clone())))
}

async fn serve(doc: OpenApi) -> Json<OpenApi> {
    Json(doc)
}
