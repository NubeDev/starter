//! Bridge from the host's authenticated [`Principal`] to the extension
//! substrate's [`CallerIdentity`], plus a task-local sidecar so the
//! rubix-side authz backend can read the *full* `Principal` without a
//! signature widen on the substrate `RestDispatcher` trait.
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
//!
//! # Why also a task-local?
//!
//! `CallerIdentity` is the substrate's transport-neutral identity
//! shape: `tenant_id`, `user_id`, `Vec<String>` of role labels. The
//! rubix authz engine evaluates policies that may consult
//! `principal.scopes` / `.teams` / `.extra` — fields the
//! `CallerIdentity` flattening discards. Rather than widen
//! `RestDispatcher::dispatch(..., caller: Option<CallerIdentity>)`
//! to also carry a `Principal` (substrate gains no value from
//! knowing rubix's auth shape), we stash an `Arc<Principal>` in a
//! task-local. The rubix [`super::RubixCapabilityFactory::authz`]
//! impl reads it via [`current_principal`] at per-call backend
//! construction (which runs on the request task) and threads the
//! full principal into [`super::RubixAuthzBackend`]. The substrate
//! stays unaware. See `crates/starter-mcp/src/principal_local.rs`
//! for the same pattern in the MCP transport.

use std::future::Future;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::middleware::{from_fn, Next};
use axum::response::Response;
use axum::Router;

use starter_ext_spi::identity::CallerIdentity;
use starter_spi::auth::Principal;

tokio::task_local! {
    /// Per-request `Principal` made visible to the rubix-side
    /// capability backends. Set by [`with_caller_identity`]; read
    /// by [`current_principal`].
    static REQUEST_PRINCIPAL: Arc<Principal>;
}

/// Run `fut` with `principal` bound on the current task. Surfaced
/// for unit tests that want to exercise capability backends without
/// standing up an axum stack.
pub async fn with_request_principal<F, T>(principal: Arc<Principal>, fut: F) -> T
where
    F: Future<Output = T>,
{
    REQUEST_PRINCIPAL.scope(principal, fut).await
}

/// Return the principal bound on the current task, if any. Returns
/// `None` from non-HTTP entry points (CLI, scheduler) and from any
/// task that wasn't entered via [`with_caller_identity`].
pub fn current_principal() -> Option<Arc<Principal>> {
    REQUEST_PRINCIPAL.try_with(Arc::clone).ok()
}

/// Wrap `router` with a layer that materialises a [`CallerIdentity`]
/// from the inbound [`Principal`] and binds the `Principal` itself
/// as a task-local for downstream capability-factory reads. Apply
/// *inside* the `with_principal` layer (i.e. after it has had a
/// chance to insert the principal) so this layer always observes
/// the freshly attached extension.
pub fn with_caller_identity<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(inject))
}

async fn inject(mut req: Request<Body>, next: Next) -> Response {
    // Lift the `Principal` out of the request extensions if
    // `with_principal` placed one there. Clone so we can also
    // insert a `CallerIdentity` view derived from it.
    let principal = req.extensions().get::<Principal>().cloned();
    if let Some(p) = &principal {
        let caller = CallerIdentity {
            tenant_id: p.tenant_id.clone(),
            user_id: Some(p.subject.clone()),
            roles: vec![format!("{:?}", p.role)],
            request_id: String::new(),
        };
        req.extensions_mut().insert(caller);
    }
    // Bind the principal as a task-local for the lifetime of the
    // downstream handler chain. Anonymous / unauthenticated
    // requests skip the scope entirely — `current_principal()`
    // returns `None`, and the rubix capability factory falls
    // through to its principal-from-CallerIdentity reconstruction.
    match principal {
        Some(p) => with_request_principal(Arc::new(p), next.run(req)).await,
        None => next.run(req).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_spi::auth::Role;

    #[test]
    fn current_principal_returns_none_outside_scope() {
        assert!(current_principal().is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn with_request_principal_binds_task_local() {
        let p = Principal {
            subject: "u-1".into(),
            role: Role::Admin,
            scopes: Vec::new(),
            tenant_id: Some("t-1".into()),
            teams: Vec::new(),
            tenant_scope: Vec::new(),
            extra: serde_json::Value::Null,
        };
        let arc = Arc::new(p);
        let observed = with_request_principal(arc.clone(), async move {
            current_principal().map(|inner| inner.subject.clone())
        })
        .await;
        assert_eq!(observed.as_deref(), Some("u-1"));
    }
}
