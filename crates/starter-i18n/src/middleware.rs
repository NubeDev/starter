//! Tower middleware: parses `Accept-Language`, walks the SCOPE R5
//! fallback chain via [`crate::locale::pick_language`], and threads
//! the resolved [`LocaleCtx`] into request extensions for downstream
//! handlers.
//!
//! Owns: SCOPE.md Phase 3 "Middleware" — `Accept-Language` negotiation
//! and the opt-in `X-I18n-Fallback` debug header.
//!
//! # What the middleware does
//!
//! 1. Parses the request's `Accept-Language` header (missing → empty
//!    string → fallback walk returns the bundle's static fallback).
//! 2. Calls [`crate::locale::pick_language`] against the languages the
//!    bound [`MessageBundle`] knows about; the result is the
//!    [`LanguageTag`] handlers should render with.
//! 3. Inserts a [`LocaleCtx`] into request extensions (cheap to clone
//!    — one `LanguageTag` plus a fallback flag).
//! 4. Sets `Content-Language: <chosen>` and appends `Accept-Language`
//!    to the response `Vary` header so caches key the response on the
//!    negotiation axis.
//! 5. If the chosen tag is NOT an exact match for any single entry of
//!    the parsed `Accept-Language` header (i.e. the request fell
//!    through the family / wildcard / static-fallback path), emits a
//!    `tracing::debug!` event tagged `i18n.fallback`. When the layer
//!    was built with [`AcceptLanguageLayer::with_fallback_header`] the
//!    response also carries an `X-I18n-Fallback: <chosen>` header.
//!    Per SCOPE R5 the header is **off by default** — it is meant for
//!    debug clients, not production wire shapes.
//!
//! # What the middleware does NOT do
//!
//! No response body rewriting. The chosen language is exposed via the
//! request extension and downstream handlers (or the Phase 5
//! diagnostics rewriter, opt-in via its own feature gate) decide how
//! to render. SSE / streaming bodies are explicitly out of scope per
//! SCOPE R5.

use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use futures::future::BoxFuture;
use http::header::{HeaderName, HeaderValue, ACCEPT_LANGUAGE, CONTENT_LANGUAGE, VARY};
use starter_spi::i18n::LanguageTag;
use tower::{Layer, Service};

use crate::bundle::MessageBundle;
use crate::locale::{parse_accept_language, pick_language};

/// `X-I18n-Fallback` — opt-in debug header. Off by default per SCOPE
/// R5; enable via [`AcceptLanguageLayer::with_fallback_header`].
pub const X_I18N_FALLBACK: HeaderName = HeaderName::from_static("x-i18n-fallback");

/// Per-request locale context. Stashed in request extensions by
/// [`accept_language_layer`]; handlers read it via
/// `axum::Extension<LocaleCtx>`.
#[derive(Debug, Clone)]
pub struct LocaleCtx {
    chosen: LanguageTag,
    fallback: bool,
}

impl LocaleCtx {
    /// The [`LanguageTag`] the negotiation picked — the tag handlers
    /// should render with.
    #[must_use]
    pub fn language(&self) -> &LanguageTag {
        &self.chosen
    }

    /// `true` if the chosen tag did NOT match any exact entry in the
    /// parsed `Accept-Language` list (the request fell through the
    /// family / wildcard / static-fallback path).
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        self.fallback
    }
}

/// Tower [`Layer`] for `Accept-Language` negotiation. Build via
/// [`accept_language_layer`] and (optionally) chain
/// [`AcceptLanguageLayer::with_fallback_header`] to opt into the debug
/// header.
#[derive(Clone)]
pub struct AcceptLanguageLayer {
    bundle: Arc<MessageBundle>,
    available: Arc<Vec<LanguageTag>>,
    fallback_header: bool,
}

/// Build the `Accept-Language` layer. `bundle` is the source of truth
/// for both the available-languages list (used by the family /
/// wildcard fallback walk) and the static fallback tag (used by R5
/// step 3).
///
/// The list of available languages is captured **once** at layer-build
/// time. Hot-swapping the underlying [`MessageBundle`] (e.g. via an
/// `Arc` reload) is fine for translation lookups but will NOT extend
/// the negotiation surface — callers that need that should rebuild
/// the layer.
pub fn accept_language_layer(bundle: Arc<MessageBundle>) -> AcceptLanguageLayer {
    let mut available: Vec<LanguageTag> = bundle.languages().cloned().collect();
    // Deterministic order so the wildcard / family-tie fallback picks
    // the same tag across processes — see `pick_language` docs.
    available.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    AcceptLanguageLayer {
        bundle,
        available: Arc::new(available),
        fallback_header: false,
    }
}

impl AcceptLanguageLayer {
    /// Opt into the `X-I18n-Fallback: <lang>` response header for
    /// requests that walked the R5 fallback chain. Off by default per
    /// SCOPE R5; the header is intended for debug clients.
    #[must_use]
    pub fn with_fallback_header(mut self, enabled: bool) -> Self {
        self.fallback_header = enabled;
        self
    }
}

impl<S> Layer<S> for AcceptLanguageLayer {
    type Service = AcceptLanguageService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AcceptLanguageService {
            inner,
            bundle: self.bundle.clone(),
            available: self.available.clone(),
            fallback_header: self.fallback_header,
        }
    }
}

/// Tower [`Service`] produced by [`AcceptLanguageLayer`].
#[derive(Clone)]
pub struct AcceptLanguageService<S> {
    inner: S,
    bundle: Arc<MessageBundle>,
    available: Arc<Vec<LanguageTag>>,
    fallback_header: bool,
}

impl<S> Service<Request> for AcceptLanguageService<S>
where
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Response, Infallible>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        // Standard tower pattern — clone the poll-ready inner into the
        // future, swap a fresh clone back into `self` for the next
        // call's `poll_ready` cycle.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let bundle = self.bundle.clone();
        let available = self.available.clone();
        let fallback_header = self.fallback_header;

        Box::pin(async move {
            let (mut parts, body) = req.into_parts();

            let raw = parts
                .headers
                .get(ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            let chosen =
                pick_language(available.as_ref(), &raw, bundle.fallback().clone());

            // Determine whether the chosen tag was an exact match for
            // any quality-weighted entry of the parsed list. If so
            // the request did NOT fall through — anything else is
            // the fallback path per R5.
            let is_fallback = !parse_accept_language(&raw)
                .iter()
                .any(|(tag, _)| tag.as_str() == chosen.as_str());

            if is_fallback {
                tracing::debug!(
                    target: "i18n.fallback",
                    chosen = %chosen,
                    accept_language = %raw,
                    "Accept-Language fell through to fallback chain",
                );
            }

            parts.extensions.insert(LocaleCtx {
                chosen: chosen.clone(),
                fallback: is_fallback,
            });
            let req = Request::from_parts(parts, body);

            let mut resp = inner.call(req).await?;
            set_content_language(&mut resp, chosen.as_str());
            append_vary(&mut resp, "Accept-Language");
            if is_fallback && fallback_header {
                if let Ok(hv) = HeaderValue::from_str(chosen.as_str()) {
                    resp.headers_mut().insert(X_I18N_FALLBACK, hv);
                }
            }
            Ok(resp)
        })
    }
}

/// Insert a `Content-Language` header, replacing any value the
/// handler set. The middleware-resolved tag is the authoritative one
/// for the negotiated response.
fn set_content_language(resp: &mut Response<Body>, value: &str) {
    if let Ok(hv) = HeaderValue::from_str(value) {
        resp.headers_mut().insert(CONTENT_LANGUAGE, hv);
    }
}

/// Append `value` to the response `Vary` header without clobbering
/// pre-existing entries. Mirrors the helper in
/// `starter-server::middleware::accept_units` — kept local here so
/// `starter-i18n` does not depend on `starter-server`.
fn append_vary(resp: &mut Response<Body>, value: &str) {
    let headers = resp.headers_mut();
    if let Some(existing) = headers.get(VARY) {
        if let Ok(existing_str) = existing.to_str() {
            if existing_str
                .split(',')
                .any(|p| p.trim().eq_ignore_ascii_case(value))
            {
                return;
            }
            if let Ok(new) = HeaderValue::from_str(&format!("{existing_str}, {value}")) {
                headers.insert(VARY, new);
                return;
            }
        }
    }
    if let Ok(hv) = HeaderValue::from_str(value) {
        headers.insert(VARY, hv);
    }
}

// ---------------------------------------------------------------------
// Integration tests.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Extension;
    use axum::routing::get;
    use axum::Router;
    use http::{Request as HttpRequest, StatusCode};

    use crate::catalog::Catalog;
    use starter_spi::i18n::MessageKey;
    use tower::ServiceExt;

    fn tag(s: &str) -> LanguageTag {
        LanguageTag::parse(s).expect("test tag must parse")
    }

    fn key(s: &str) -> MessageKey {
        MessageKey::parse(s).expect("test key must parse")
    }

    fn bundle() -> Arc<MessageBundle> {
        let mut b = MessageBundle::new(tag("en"));
        let mut en = std::collections::BTreeMap::new();
        en.insert(key("a.b"), "english".to_string());
        b.insert(tag("en"), Catalog { messages: en });
        let mut es = std::collections::BTreeMap::new();
        es.insert(key("a.b"), "spanish".to_string());
        b.insert(tag("es"), Catalog { messages: es });
        Arc::new(b)
    }

    async fn handler(Extension(ctx): Extension<LocaleCtx>) -> String {
        format!("{}|{}", ctx.language(), ctx.is_fallback())
    }

    fn app(b: Arc<MessageBundle>, with_header: bool) -> Router {
        let layer = accept_language_layer(b).with_fallback_header(with_header);
        Router::new().route("/t", get(handler)).layer(layer)
    }

    async fn body_string(resp: Response) -> String {
        use http_body_util::BodyExt;
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn exact_match_sets_content_language_and_vary_no_fallback_header() {
        let app = app(bundle(), true);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/t")
                    .header(ACCEPT_LANGUAGE, "es")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_LANGUAGE).unwrap().to_str().unwrap(),
            "es"
        );
        let vary = resp.headers().get(VARY).unwrap().to_str().unwrap();
        assert!(vary
            .split(',')
            .any(|p| p.trim().eq_ignore_ascii_case("Accept-Language")));
        assert!(resp.headers().get(X_I18N_FALLBACK).is_none());
        assert_eq!(body_string(resp).await, "es|false");
    }

    #[tokio::test]
    async fn missing_header_falls_back_to_en_and_sets_x_i18n_fallback_when_enabled() {
        let app = app(bundle(), true);
        let resp = app
            .oneshot(HttpRequest::builder().uri("/t").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_LANGUAGE).unwrap().to_str().unwrap(),
            "en"
        );
        let xfb = resp.headers().get(X_I18N_FALLBACK).expect("X-I18n-Fallback present");
        assert_eq!(xfb.to_str().unwrap(), "en");
        assert_eq!(body_string(resp).await, "en|true");
    }

    #[tokio::test]
    async fn fallback_header_off_by_default() {
        let app = app(bundle(), false);
        let resp = app
            .oneshot(HttpRequest::builder().uri("/t").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Fell back to en, but the header is off by default.
        assert!(resp.headers().get(X_I18N_FALLBACK).is_none());
    }

    #[tokio::test]
    async fn family_fallback_emits_x_i18n_fallback_when_enabled() {
        // Request en-US; bundle has en — family fallback applies.
        let app = app(bundle(), true);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/t")
                    .header(ACCEPT_LANGUAGE, "en-US")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.headers().get(CONTENT_LANGUAGE).unwrap().to_str().unwrap(),
            "en"
        );
        let xfb = resp.headers().get(X_I18N_FALLBACK).expect("X-I18n-Fallback present");
        assert_eq!(xfb.to_str().unwrap(), "en");
    }
}
