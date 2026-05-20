//! `with_permission(router, kind, action)` — the axum middleware
//! that turns a [`starter_spi::authz::PolicyEngine`] into a route-
//! level gate.
//!
//! Two enforcement points (SCOPE.md R5): the middleware is the
//! cheap, declarative path; handlers call `engine.check` directly
//! for the row-level refinement once the row has been loaded
//! (that is when `object.owner` becomes available).
//!
//! Layer order: wrap this **inside** whichever middleware attaches
//! the [`starter_spi::auth::Principal`] to the request extensions.
//! The engine must also be present in extensions as
//! `Arc<dyn PolicyEngine>` — typically inserted once at router
//! build time:
//!
//! ```ignore
//! use std::sync::Arc;
//! use axum::Extension;
//! use starter_spi::authz::PolicyEngine;
//!
//! let router = router.layer(Extension(engine.clone() as Arc<dyn PolicyEngine>));
//! ```

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};

use starter_spi::auth::Principal;
use starter_spi::authz::{Decision, PolicyEngine, ResourceRef};

/// Wrap `router` in a permission gate. Issues a collection-level
/// check (`ResourceRef { id: None, owner: None }`). For row-level
/// checks where ownership matters, call `engine.check(..)`
/// directly inside the handler after loading the row.
///
/// Mirrors the `with_role` ergonomics already used by
/// `starter-server::auth`.
pub fn with_permission<S>(router: Router<S>, kind: &'static str, action: &'static str) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(move |req: Request<Body>, next: Next| async move {
        gate(kind, action, req, next).await
    }))
}

/// In-handler / call-site convenience: run an authorization check
/// against the engine pulled out of request extensions, mapping a
/// `Deny` to the same `403 { "error": <reason> }` body the
/// middleware would emit.
///
/// Use this from inside a handler after loading the row, so the
/// caller can pass `object.owner`:
///
/// ```ignore
/// async fn update_flow(
///     Extension(engine): Extension<Arc<dyn PolicyEngine>>,
///     Extension(p): Extension<Principal>,
///     Path(id): Path<String>,
///     State(s): State<AppState>,
/// ) -> Result<Json<Flow>, Response> {
///     let flow = s.flows.get(&id).await.map_err(internal)?;
///     check_or_deny(&engine, &p, "update",
///         &ResourceRef::row("flows", id).with_owner(flow.owner_id.clone())).await?;
///     // ... proceed
/// }
/// ```
pub async fn check_or_deny(
    engine: &Arc<dyn PolicyEngine>,
    principal: &Principal,
    action: &str,
    object: &ResourceRef,
) -> std::result::Result<(), Response> {
    match engine.check(principal, action, object).await {
        Decision::Allow { .. } => Ok(()),
        Decision::Deny { reason, .. } => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": reason})),
        )
            .into_response()),
    }
}

async fn gate(
    kind: &'static str,
    action: &'static str,
    req: Request<Body>,
    next: Next,
) -> Response {
    let principal = match req.extensions().get::<Principal>() {
        Some(p) => p.clone(),
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let engine = match req.extensions().get::<Arc<dyn PolicyEngine>>() {
        Some(e) => e.clone(),
        None => {
            // No engine wired in — deny loudly. SCOPE.md R3:
            // missing infrastructure fails closed.
            tracing::error!(
                kind = %kind,
                action = %action,
                "authz: PolicyEngine extension missing on request"
            );
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "engine_missing"})),
            )
                .into_response();
        }
    };

    let object = ResourceRef::collection(kind);
    match engine.check(&principal, action, &object).await {
        Decision::Allow { .. } => next.run(req).await,
        Decision::Deny { reason, .. } => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": reason})),
        )
            .into_response(),
    }
}

/// Re-exported alias so callers can write
/// `.layer(require_permission(...))` matching the `require_role`
/// shape. Equivalent to constructing a single-route `Router` and
/// wrapping with [`with_permission`].
pub fn require_permission(
    kind: &'static str,
    action: &'static str,
) -> axum::middleware::FromFnLayer<
    impl Fn(
            Request<Body>,
            Next,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
        + Clone,
    (),
    (),
> {
    from_fn(move |req: Request<Body>, next: Next| {
        Box::pin(gate(kind, action, req, next))
            as std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
    })
}
