//! `with_anonymous_principal(router, principal)` — unconditionally
//! insert a fixed [`Principal`] as a request extension.
//!
//! Intended for example binaries and single-operator deployments that
//! have **no real authentication** but want to mount routes that
//! require a `Principal` (e.g. `starter-prefs`'
//! `/v1/me/preferences`). The injected principal is identical on
//! every request — there is no transport-level authentication.
//!
//! # When NOT to use this
//!
//! This layer is a deliberate bypass of the `Authenticator` trait.
//! Production binaries with real users must use
//! [`super::with_principal`] paired with a real `Authenticator`
//! implementation (`starter-auth-token`, `starter-auth-users`,
//! `starter-auth-oauth`, …). Mixing the two on the same router is
//! safe (the anonymous layer runs after `with_principal` and only
//! inserts if no principal is already present), but the anonymous
//! fallback effectively neutralises any downstream `with_role` /
//! `with_scope` guards because the injected principal carries
//! whatever role the operator chose.
//!
//! # Layer order
//!
//! Mount as the **outermost** layer, exactly like
//! [`super::with_principal`]. The injection runs on the way in so
//! downstream handlers see the extension.

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::middleware::{from_fn, Next};
use axum::Router;
use starter_spi::auth::Principal;

/// Apply unconditional principal injection to `router`. The supplied
/// `principal` is cloned into the request extensions of every
/// incoming request that does NOT already carry one (so this layer
/// composes safely with [`super::with_principal`] on routers that
/// mix authenticated and anonymous traffic).
pub fn with_anonymous_principal<S>(router: Router<S>, principal: Principal) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let principal = Arc::new(principal);
    router.layer(from_fn(move |mut req: Request<Body>, next: Next| {
        let principal = principal.clone();
        async move {
            if req.extensions().get::<Principal>().is_none() {
                req.extensions_mut().insert((*principal).clone());
            }
            next.run(req).await
        }
    }))
}

/// Convenience constructor for the "local single-operator" principal
/// shape used by example binaries: a stable `subject`, [`Role::Admin`]
/// so the operator can hit admin-gated routes, and no extra scopes
/// or claims.
///
/// [`Role::Admin`]: starter_spi::auth::Role::Admin
#[must_use]
pub fn local_operator(subject: impl Into<String>) -> Principal {
    use starter_spi::auth::Role;
    Principal {
        subject: subject.into(),
        role: Role::Admin,
        scopes: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::Extension;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn echo_subject(Extension(p): Extension<Principal>) -> String {
        p.subject
    }

    #[tokio::test]
    async fn injects_when_no_principal_present() {
        let app: Router = with_anonymous_principal(
            Router::new().route("/who", get(echo_subject)),
            local_operator("local"),
        );

        let resp = app
            .oneshot(Request::builder().uri("/who").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"local");
    }

    #[tokio::test]
    async fn does_not_overwrite_existing_principal() {
        use starter_spi::auth::Role;
        let injector = from_fn(|mut req: Request<Body>, next: Next| async move {
            req.extensions_mut().insert(Principal {
                subject: "real-user".to_owned(),
                role: Role::Reader,
                scopes: Vec::new(),
                extra: serde_json::Value::Null,
            });
            next.run(req).await
        });
        let app: Router = with_anonymous_principal(
            Router::new().route("/who", get(echo_subject)),
            local_operator("local"),
        )
        .layer(injector);

        let resp = app
            .oneshot(Request::builder().uri("/who").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"real-user");
    }
}
