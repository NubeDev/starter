//! The builder: collect consumer Routers, the OpenAPI doc, and
//! configuration, and assemble a final `axum::Router`.

use axum::Router;

/// Fluent builder for the starter server.
///
/// Generic over the consumer's `AppState` so consumer Routers,
/// extractors, and handlers can use their own state type without
/// adapters.
///
/// ```ignore
/// let router = ServerBuilder::<AppState>::new(state)
///     .merge_router(domain_a::routes())
///     .merge_router(domain_b::routes())
///     .with_openapi(MyApiDoc::openapi())
///     .build();
/// ```
pub struct ServerBuilder<S> {
    state: S,
    routers: Vec<Router<S>>,
    openapi: Option<utoipa::openapi::OpenApi>,
}

impl<S: Clone + Send + Sync + 'static> ServerBuilder<S> {
    /// Start a builder with the consumer's shared state.
    pub fn new(state: S) -> Self {
        Self {
            state,
            routers: Vec::new(),
            openapi: None,
        }
    }

    /// Merge a consumer-built Router into the final assembly.
    pub fn merge_router(mut self, router: Router<S>) -> Self {
        self.routers.push(router);
        self
    }

    /// Attach the consumer's OpenAPI document. The starter-owned
    /// `/openapi.json` route serves this.
    pub fn with_openapi(mut self, doc: utoipa::openapi::OpenApi) -> Self {
        self.openapi = Some(doc);
        self
    }

    /// Materialise the final `axum::Router`.
    ///
    /// Mounts (in order): consumer routers → starter routes
    /// (`/health`, `/metrics`, `/openapi.json`) → middleware stack
    /// (CORS, tracing, request-id, latency).
    pub fn build(self) -> Router {
        // TODO(ap): finish wiring once routes/openapi/middleware
        // modules have concrete impls. Public surface is the locked
        // shape; body lands next.
        let _ = (self.state, self.routers, self.openapi);
        Router::new()
    }
}
