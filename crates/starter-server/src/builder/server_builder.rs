//! The builder: collect consumer Routers, the OpenAPI doc, and
//! the shared prometheus registry, and assemble a final `axum::Router`.

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use prometheus::Registry;
use starter_observability::metrics::StandardMetrics;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::middleware::{with_latency, with_request_id};
use crate::routes::{health_router, metrics_router, openapi_router};
use crate::static_assets;

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
///     .with_metrics(registry, standard_metrics)
///     .build();
/// ```
pub struct ServerBuilder<S> {
    state: S,
    routers: Vec<Router<S>>,
    openapi: Option<utoipa::openapi::OpenApi>,
    metrics: Option<(Arc<Registry>, Arc<StandardMetrics>)>,
    cors: Option<CorsLayer>,
    static_mounts: Vec<(String, PathBuf)>,
}

impl<S: Clone + Send + Sync + 'static> ServerBuilder<S> {
    /// Start a builder with the consumer's shared state.
    pub fn new(state: S) -> Self {
        Self {
            state,
            routers: Vec::new(),
            openapi: None,
            metrics: None,
            cors: None,
            static_mounts: Vec::new(),
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

    /// Attach the shared prometheus registry plus the standard
    /// metrics handle. Required to expose `/metrics` and to drive the
    /// latency middleware; without it neither is mounted.
    pub fn with_metrics(mut self, registry: Arc<Registry>, metrics: Arc<StandardMetrics>) -> Self {
        self.metrics = Some((registry, metrics));
        self
    }

    /// Override the default permissive CORS layer with one of the
    /// consumer's choosing. Passing `CorsLayer::very_permissive()` is
    /// the default when this is not called.
    pub fn with_cors(mut self, cors: CorsLayer) -> Self {
        self.cors = Some(cors);
        self
    }

    /// Mount a static-asset directory (built SPA bundle) at
    /// `mount_path`. Off by default. Requests that don't match a file
    /// on disk fall back to `index.html` so a client-side router owns
    /// the URL space below the mount. May be called multiple times to
    /// mount more than one bundle.
    pub fn with_static_assets(
        mut self,
        mount_path: impl Into<String>,
        dist_dir: impl Into<PathBuf>,
    ) -> Self {
        self.static_mounts
            .push((mount_path.into(), dist_dir.into()));
        self
    }

    /// Materialise the final `axum::Router`.
    ///
    /// Mount order: consumer routers (merged into one), starter-owned
    /// routes (`/health`, `/metrics`, `/openapi.json`), then the
    /// middleware stack — tracing, CORS, request-id, latency. Outer
    /// layers see requests last and responses first.
    pub fn build(self) -> Router {
        let mut app: Router<S> = self.routers.into_iter().fold(Router::new(), Router::merge);

        app = app.merge(health_router::<S>());

        if let Some((registry, _)) = &self.metrics {
            app = app.merge(metrics_router::<S>(registry.clone()));
        }
        if let Some(doc) = self.openapi {
            app = app.merge(openapi_router::<S>(doc));
        }

        // Opt-in static-asset mounts. ServeDir is stateless so this
        // happens before `with_state` for parity with consumer routers.
        for (mount_path, dist_dir) in self.static_mounts {
            app = static_assets::mount(app, &mount_path, dist_dir);
        }

        // Apply the consumer state so the result is `Router<()>` —
        // axum requires a fully-resolved router before `serve`.
        let mut app = app.with_state(self.state);

        // Middleware stack. Order matters: layers applied later are
        // outermost (see the request first, the response last).
        let cors = self.cors.unwrap_or_else(CorsLayer::very_permissive);
        app = with_request_id(app)
            .layer(cors)
            .layer(TraceLayer::new_for_http());
        if let Some((_, metrics)) = self.metrics {
            app = with_latency(app, metrics);
        }
        app
    }
}
