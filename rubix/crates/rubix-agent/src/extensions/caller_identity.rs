//! Bridge from the host's authenticated [`Principal`] to the extension
//! substrate's [`CallerIdentity`].
//!
//! `starter-server::auth::with_principal` puts a `Principal` into the
//! request extensions; the extension-substrate handlers read a
//! `CallerIdentity` from the same bag (see
//! `starter-ext-server/src/rest/handler.rs`). This layer runs after
//! `with_principal` so the `Principal` is already attached, lifts the
//! tenancy / user / role fields into a `CallerIdentity`, and inserts
//! it for the downstream `non_streaming` / `sse` / `ndjson` handlers
//! to consume. A request that never carried credentials never gets a
//! `Principal`, so we never insert a `CallerIdentity` either — the
//! handlers see `None` and the per-call capability factory falls
//! through to its fail-closed defaults (no tenant, no namespace).

use axum::body::Body;
use axum::http::Request;
use axum::middleware::{from_fn, Next};
use axum::response::Response;
use axum::Router;

use starter_ext_spi::identity::CallerIdentity;
use starter_spi::auth::Principal;

/// Wrap `router` with a layer that materialises a [`CallerIdentity`]
/// from the inbound [`Principal`]. Apply *inside* the
/// `with_principal` layer (i.e. after it has had a chance to insert
/// the principal) so this layer always observes the freshly attached
/// extension.
pub fn with_caller_identity<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(inject))
}

async fn inject(mut req: Request<Body>, next: Next) -> Response {
    if let Some(principal) = req.extensions().get::<Principal>().cloned() {
        let caller = CallerIdentity {
            tenant_id: principal.tenant_id.clone(),
            user_id: Some(principal.subject.clone()),
            roles: vec![format!("{:?}", principal.role)],
            request_id: String::new(),
        };
        req.extensions_mut().insert(caller);
    }
    next.run(req).await
}
