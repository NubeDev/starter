//! v3 — `tower::Layer` for core HTTP routes (§Layer 2 / Layer C).
//!
//! `CacheLayer::tower()` returns an `axum`-compatible `tower::Layer`
//! wrapping a JSON-returning service. The layer derives the cache
//! key from `(route_path, query_string, scope-vars)` and serves
//! cached bytes on hit. Specs are passed through the same
//! [`CacheSpec`] shape — the author surface does not change between
//! integration points.
//!
//! Gated behind feature `tower` so consumers that only want the
//! kind-dispatcher integration don't pull `tower-layer` into their
//! tree.

use crate::layer::{Bytes, CacheLayer, CallerScope};
use crate::spec::CacheSpec;
use http::{Request, Response};
use http_body::Body;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower_layer::Layer;
use tower_service::Service;

/// A pre-built `tower::Layer` that wraps a route in cache lookups.
///
/// Construction is via [`CacheLayer::tower`].
#[derive(Clone)]
pub struct TowerCacheLayer {
    inner: Arc<TowerInner>,
}

struct TowerInner {
    layer: CacheLayer,
    spec: CacheSpec,
    spec_id: String,
}

impl TowerCacheLayer {
    /// Build a tower layer from a [`CacheLayer`] + a [`CacheSpec`] +
    /// a route id (used as the per-spec stats label).
    pub fn new(layer: CacheLayer, spec: CacheSpec, spec_id: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(TowerInner {
                layer,
                spec,
                spec_id: spec_id.into(),
            }),
        }
    }
}

impl CacheLayer {
    /// Build a tower-compatible layer that wraps a route in the same
    /// cache machinery the kind dispatcher uses. The key is derived
    /// from `(method, route, query string)`. Per-tenant / per-user
    /// scoping comes from the request headers `x-tenant-id` /
    /// `x-user-id` if present (the host's auth middleware is expected
    /// to set these); otherwise system scope is used.
    pub fn tower(&self, spec: CacheSpec, spec_id: impl Into<String>) -> TowerCacheLayer {
        TowerCacheLayer::new(self.clone(), spec, spec_id)
    }
}

impl<S> Layer<S> for TowerCacheLayer {
    type Service = TowerCacheService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        TowerCacheService {
            inner,
            cache: self.inner.clone(),
        }
    }
}

/// The wrapped service.
#[derive(Clone)]
pub struct TowerCacheService<S> {
    inner: S,
    cache: Arc<TowerInner>,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for TowerCacheService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + Sync + 'static,
    ReqBody: Send + 'static,
    ResBody: Body<Data = bytes::Bytes> + Send + 'static,
    ResBody::Error: std::fmt::Display + Send,
{
    type Response = Response<http_body_util::Full<bytes::Bytes>>;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Self::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let cache = self.cache.clone();
        let mut inner = self.inner.clone();
        Box::pin(async move {
            // Key parts.
            let method = req.method().clone();
            let path = req.uri().path().to_string();
            let query = req.uri().query().unwrap_or("").to_string();
            let tenant = req
                .headers()
                .get("x-tenant-id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let user = req
                .headers()
                .get("x-user-id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let caller = match (tenant, user) {
                (Some(t), Some(u)) => CallerScope::new(t, u),
                (Some(t), None) => CallerScope {
                    tenant: Some(t),
                    user: None,
                },
                _ => CallerScope::system(),
            };
            let base_key = format!("{method} {path}?{query}");

            let layer = cache.layer.clone();
            let spec = cache.spec.clone();
            let spec_id = cache.spec_id.clone();
            let req_holder: std::sync::Mutex<Option<Request<ReqBody>>> =
                std::sync::Mutex::new(Some(req));

            let result: Result<Bytes, ServiceFailure<S::Error>> = layer
                .get_or_load_labelled(
                    &spec,
                    Some(spec_id.as_str()),
                    &caller,
                    &base_key,
                    || async {
                        let req = req_holder.lock().unwrap().take().expect("once");
                        let resp = inner.call(req).await.map_err(ServiceFailure::Inner)?;
                        let (_parts, body) = resp.into_parts();
                        let collected = http_body_util::BodyExt::collect(body)
                            .await
                            .map_err(|e| ServiceFailure::Body(e.to_string()))?;
                        let b = collected.to_bytes();
                        Ok(Arc::new(b.to_vec()))
                    },
                )
                .await;
            match result {
                Ok(bytes) => {
                    let body = http_body_util::Full::new(bytes::Bytes::from((*bytes).clone()));
                    Ok(Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(body)
                        .expect("response build"))
                }
                Err(ServiceFailure::Inner(e)) => Err(e),
                Err(ServiceFailure::Body(_)) => {
                    let body = http_body_util::Full::new(bytes::Bytes::new());
                    Ok(Response::builder()
                        .status(502)
                        .body(body)
                        .expect("response build"))
                }
            }
        })
    }
}

enum ServiceFailure<E> {
    Inner(E),
    Body(#[allow(dead_code)] String),
}
