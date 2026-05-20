//! Scope-limited diagnostics rewriter — SCOPE.md Phase 5.
//!
//! The rewriter is the **only** part of `starter-i18n` that mutates
//! response bodies. It is default-off (feature `diagnostics`), opt-in
//! per handler, and intentionally narrow:
//!
//! - It runs **only** when the handler inserts a [`DiagnosticBody`]
//!   into the response extensions. Absence of the extension is a
//!   no-op — the body is forwarded unchanged.
//! - It rewrites **only** the documented envelope shape at the
//!   documented top-level paths (SCOPE D-5.1):
//!   - the response body's top-level `"diagnostic"` object
//!     (`{ "code": "<MessageKey>", "params": { … } }`), and
//!   - each element of the top-level `"diagnostics"` array (same
//!     shape).
//!   Nothing else in the body is read, walked, or rewritten.
//! - **SSE / chunked / streaming responses are NEVER rewritten** per
//!   SCOPE R5. The layer inspects `Content-Type` and
//!   `Transfer-Encoding` on the way out and bails on
//!   `text/event-stream` or `chunked` regardless of whether the
//!   handler opted in. Clients translate per-event in that case.
//! - The bundle is supplied at layer-build time. `LocaleCtx` is read
//!   from request extensions (typically populated upstream by
//!   [`crate::middleware::accept_language_layer`]); if the locale
//!   middleware is not chained, the rewriter falls back to the
//!   bundle's static fallback language so it is still well-behaved.
//! - **Missing translations are not fatal.** When the bundle has no
//!   translation for an envelope's [`MessageKey`], that envelope is
//!   left unchanged (no `message` field is added); the rest of the
//!   body is forwarded untouched. The consumer still sees the
//!   structured `{code, params}` payload and can translate locally.
//!
//! # Rewritten shape
//!
//! Each translated envelope gains a `"message"` field with the
//! rendered translation. The `"code"` and `"params"` fields are
//! preserved verbatim so consumers that ignore `Content-Language`
//! continue to see the structured payload they would have seen
//! without the rewriter in the chain.
//!
//! Before (handler emits):
//!
//! ```json
//! { "diagnostic": { "code": "auth.token.expired", "params": {} } }
//! ```
//!
//! After (rewriter translates):
//!
//! ```json
//! {
//!   "diagnostic": {
//!     "code": "auth.token.expired",
//!     "params": {},
//!     "message": "Your session has expired."
//!   }
//! }
//! ```

use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use bytes::Bytes;
use futures::future::BoxFuture;
use http::header::{HeaderValue, CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http_body_util::BodyExt;
use serde_json::{Map, Value};
use starter_spi::i18n::{LanguageTag, MessageKey};
use tower::{Layer, Service};

use crate::bundle::MessageBundle;
#[cfg(feature = "routes")]
use crate::middleware::LocaleCtx;

/// Marker handlers insert into the response extensions to opt the
/// response into translation. Presence enables the rewriter;
/// absence is a no-op.
///
/// Zero-sized so insertion is free; the type identity is what the
/// rewriter looks for.
///
/// ```ignore
/// async fn login_failed() -> Response {
///     let mut resp = (
///         StatusCode::UNAUTHORIZED,
///         Json(json!({ "diagnostic": { "code": "auth.token.expired", "params": {} } })),
///     ).into_response();
///     resp.extensions_mut().insert(DiagnosticBody::new());
///     resp
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct DiagnosticBody;

impl DiagnosticBody {
    /// Construct the marker. The type carries no state.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Build the [`DiagnosticsLayer`]. The supplied bundle is the source
/// of truth for translations; the layer holds an `Arc` and clones
/// cheaply.
#[must_use]
pub fn diagnostics_layer(bundle: Arc<MessageBundle>) -> DiagnosticsLayer {
    DiagnosticsLayer { bundle }
}

/// Tower [`Layer`] for the Phase 5 diagnostics rewriter.
#[derive(Clone)]
pub struct DiagnosticsLayer {
    bundle: Arc<MessageBundle>,
}

impl<S> Layer<S> for DiagnosticsLayer {
    type Service = DiagnosticsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DiagnosticsService {
            inner,
            bundle: self.bundle.clone(),
        }
    }
}

/// Tower [`Service`] produced by [`DiagnosticsLayer`].
#[derive(Clone)]
pub struct DiagnosticsService<S> {
    inner: S,
    bundle: Arc<MessageBundle>,
}

impl<S> Service<Request> for DiagnosticsService<S>
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
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let bundle = self.bundle.clone();

        // Pull the chosen language out of the request *before* we
        // hand off ownership to the inner service. The Phase 3
        // `accept_language_layer` is expected upstream; if it is
        // not chained we fall back to the bundle's static
        // fallback so the rewriter remains well-behaved in
        // isolation.
        let language = locale_from_extensions(req.extensions(), &bundle);

        Box::pin(async move {
            let resp = inner.call(req).await?;
            Ok(maybe_rewrite(resp, &bundle, &language).await)
        })
    }
}

/// Read the chosen [`LanguageTag`] from request extensions. Falls
/// back to the bundle's static fallback if no `LocaleCtx` is
/// present (the locale middleware was not chained upstream).
fn locale_from_extensions(
    extensions: &http::Extensions,
    bundle: &MessageBundle,
) -> LanguageTag {
    #[cfg(feature = "routes")]
    {
        if let Some(ctx) = extensions.get::<LocaleCtx>() {
            return ctx.language().clone();
        }
    }
    // The reference is unused when the `routes` feature is off; the
    // explicit `_` keeps clippy quiet either way.
    let _ = extensions;
    bundle.fallback().clone()
}

/// Inspect the response. If the handler opted in (`DiagnosticBody`
/// present), the body is JSON, and the response is not a streaming
/// shape we cannot safely buffer, attempt the rewrite. Otherwise
/// forward the response unchanged.
async fn maybe_rewrite(
    resp: Response,
    bundle: &MessageBundle,
    language: &LanguageTag,
) -> Response {
    // 1. The handler must have opted in. Absence is a no-op per
    //    SCOPE D-5.1.
    if resp.extensions().get::<DiagnosticBody>().is_none() {
        return resp;
    }

    // 2. SSE / chunked / streaming responses are bypassed
    //    unconditionally per SCOPE R5. We probe the headers before
    //    looking at the body so we never accidentally buffer an
    //    open event stream.
    if is_streaming(&resp) {
        return resp;
    }

    // 3. The body must be JSON. We do not walk arbitrary content
    //    types; the rewriter's scope is "the documented envelope
    //    shape in the documented JSON paths".
    if !is_json(&resp) {
        return resp;
    }

    rewrite_json(resp, bundle, language).await
}

/// `true` if `Content-Type` is `text/event-stream` or
/// `Transfer-Encoding` declares `chunked`. Either means the body is
/// streamed and the rewriter must not buffer it.
fn is_streaming(resp: &Response) -> bool {
    if let Some(ct) = resp.headers().get(CONTENT_TYPE) {
        if let Ok(s) = ct.to_str() {
            // Compare the media type prefix only; parameters
            // (`charset=utf-8`) must not flip the test.
            let mime = s
                .split(';')
                .next()
                .map(|p| p.trim())
                .unwrap_or("")
                .to_ascii_lowercase();
            if mime == "text/event-stream" {
                return true;
            }
        }
    }
    if let Some(te) = resp.headers().get(TRANSFER_ENCODING) {
        if let Ok(s) = te.to_str() {
            if s.split(',')
                .any(|p| p.trim().eq_ignore_ascii_case("chunked"))
            {
                return true;
            }
        }
    }
    false
}

/// `true` if `Content-Type` is a JSON-ish media type. We accept the
/// canonical `application/json` plus any `+json` structured suffix.
fn is_json(resp: &Response) -> bool {
    let Some(ct) = resp.headers().get(CONTENT_TYPE) else {
        return false;
    };
    let Ok(s) = ct.to_str() else {
        return false;
    };
    let mime = s
        .split(';')
        .next()
        .map(|p| p.trim())
        .unwrap_or("")
        .to_ascii_lowercase();
    mime == "application/json" || mime.ends_with("+json")
}

/// Buffer the body, parse as JSON, rewrite the two documented
/// envelope shapes, and re-emit. If buffering or parsing fails the
/// response is forwarded unchanged (the rewriter is opt-in
/// best-effort, not a correctness gate).
async fn rewrite_json(
    resp: Response,
    bundle: &MessageBundle,
    language: &LanguageTag,
) -> Response {
    let (mut parts, body) = resp.into_parts();
    let collected = match body.collect().await {
        Ok(c) => c.to_bytes(),
        Err(err) => {
            tracing::debug!(
                target: "i18n.diagnostics",
                error = %err,
                "failed to buffer response body; forwarding unchanged",
            );
            return Response::from_parts(parts, Body::empty());
        }
    };

    let Ok(mut value) = serde_json::from_slice::<Value>(&collected) else {
        // Not JSON we can parse — return the bytes unchanged. This
        // is defensive; `is_json` already gated us in on the
        // Content-Type, but a misadvertised body shouldn't 500.
        return Response::from_parts(parts, Body::from(collected));
    };

    // The two documented top-level paths (SCOPE D-5.1):
    if let Value::Object(map) = &mut value {
        if let Some(diag) = map.get_mut("diagnostic") {
            rewrite_envelope(diag, bundle, language);
        }
        if let Some(Value::Array(list)) = map.get_mut("diagnostics") {
            for item in list.iter_mut() {
                rewrite_envelope(item, bundle, language);
            }
        }
    }

    let bytes = match serde_json::to_vec(&value) {
        Ok(b) => Bytes::from(b),
        Err(err) => {
            tracing::debug!(
                target: "i18n.diagnostics",
                error = %err,
                "failed to re-serialise rewritten body; forwarding original",
            );
            return Response::from_parts(parts, Body::from(collected));
        }
    };

    // Re-emit `Content-Length` if the original carried one — the
    // body's byte length may have changed.
    if parts.headers.contains_key(CONTENT_LENGTH) {
        if let Ok(hv) = HeaderValue::from_str(&bytes.len().to_string()) {
            parts.headers.insert(CONTENT_LENGTH, hv);
        }
    }

    Response::from_parts(parts, Body::from(bytes))
}

/// Translate a single `{code, params}` envelope in place. On a
/// miss the envelope is left untouched per SCOPE D-5.1 ("missing
/// translation leaves the envelope intact").
fn rewrite_envelope(value: &mut Value, bundle: &MessageBundle, language: &LanguageTag) {
    let Value::Object(obj) = value else {
        // The handler emitted something that isn't the documented
        // shape; the rewriter is strict per R5 and leaves it.
        return;
    };

    let Some(code_val) = obj.get("code") else {
        return;
    };
    let Some(code_str) = code_val.as_str() else {
        return;
    };
    let Ok(key) = MessageKey::parse(code_str) else {
        return;
    };

    let Some(template) = bundle.lookup(language, &key) else {
        tracing::debug!(
            target: "i18n.diagnostics.miss",
            lang = %language,
            code = %key,
            "no translation; leaving envelope intact",
        );
        return;
    };

    let rendered = interpolate(template, obj.get("params"));
    obj.insert("message".to_string(), Value::String(rendered));
}

/// Substitute `{name}` placeholders in `template` with the matching
/// entries from a `DiagnosticParam` map serialised as
/// `{ "<name>": { "<variant>": <value> } }`. Unknown placeholders
/// are left literal so a template referencing a param that wasn't
/// supplied surfaces as text rather than silently dropping.
///
/// This is intentionally narrow — the SCOPE Phase 5 contract is the
/// envelope shape, not full ICU MessageFormat semantics. The
/// formatter only handles named substitution; plural / select /
/// number formatting is the React layer's job (the same templates
/// hand off to `react-intl` client-side).
fn interpolate(template: &str, params: Option<&Value>) -> String {
    let params: Option<&Map<String, Value>> = params.and_then(Value::as_object);
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            // Collect until the matching '}'. Bail if we never find
            // one — emit the original characters so a malformed
            // template survives.
            let mut name = String::new();
            let mut closed = false;
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == '}' {
                    closed = true;
                    break;
                }
                name.push(nc);
            }
            if !closed {
                out.push('{');
                out.push_str(&name);
                continue;
            }
            match params.and_then(|m| m.get(name.as_str())) {
                Some(v) => render_param(v, &mut out),
                None => {
                    out.push('{');
                    out.push_str(&name);
                    out.push('}');
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Render a single `DiagnosticParam` (serialised externally-tagged
/// as `{"string": …}`, `{"i64": …}`, etc) into the output string.
/// Unknown shapes are stringified via `to_string()` so a forward-
/// compatible variant degrades gracefully.
fn render_param(value: &Value, out: &mut String) {
    if let Value::Object(map) = value {
        if let Some((_, inner)) = map.iter().next() {
            match inner {
                Value::String(s) => out.push_str(s),
                Value::Number(n) => out.push_str(&n.to_string()),
                Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
                Value::Null => {}
                other => out.push_str(&other.to_string()),
            }
            return;
        }
    }
    out.push_str(&value.to_string());
}

// ---------------------------------------------------------------------
// Integration tests.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::routing::get;
    use axum::Router;
    use http::{Request as HttpRequest, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::json;

    use crate::catalog::Catalog;

    fn tag(s: &str) -> LanguageTag {
        LanguageTag::parse(s).expect("test tag must parse")
    }

    fn key(s: &str) -> MessageKey {
        MessageKey::parse(s).expect("test key must parse")
    }

    fn bundle() -> Arc<MessageBundle> {
        let mut b = MessageBundle::new(tag("en"));
        let mut en = std::collections::BTreeMap::new();
        en.insert(key("auth.token.expired"), "Your session has expired.".to_string());
        en.insert(key("with.param"), "Hello, {name}!".to_string());
        b.insert(tag("en"), Catalog { messages: en });
        let mut es = std::collections::BTreeMap::new();
        es.insert(key("auth.token.expired"), "Tu sesión ha expirado.".to_string());
        es.insert(key("with.param"), "Hola, {name}!".to_string());
        b.insert(tag("es"), Catalog { messages: es });
        Arc::new(b)
    }

    async fn body_value(resp: Response) -> Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).expect("test response body must be JSON")
    }

    async fn body_text(resp: Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).expect("test response body must be utf-8")
    }

    /// Build a response opting into rewriting at the body.
    fn opted_in_response(payload: Value) -> Response {
        let mut resp = (
            StatusCode::OK,
            [(CONTENT_TYPE, "application/json")],
            payload.to_string(),
        )
            .into_response();
        resp.extensions_mut().insert(DiagnosticBody::new());
        resp
    }

    /// Build a response that does NOT opt into rewriting.
    fn untouched_response(payload: Value) -> Response {
        (
            StatusCode::OK,
            [(CONTENT_TYPE, "application/json")],
            payload.to_string(),
        )
            .into_response()
    }

    use axum::response::IntoResponse;

    // `accept_language_layer` is the upstream the rewriter expects;
    // tests chain both so `LocaleCtx` is populated naturally.
    fn app_with_locale(payload: Value, opt_in: bool, lang: &'static str) -> Router {
        let b = bundle();
        let handler = move || {
            let payload = payload.clone();
            async move {
                if opt_in {
                    opted_in_response(payload)
                } else {
                    untouched_response(payload)
                }
            }
        };

        Router::new()
            .route("/d", get(handler))
            .layer(diagnostics_layer(b.clone()))
            // The Accept-Language middleware lives upstream so its
            // `LocaleCtx` lands in request extensions *before* the
            // diagnostics layer runs.
            .layer(crate::middleware::accept_language_layer(b))
            // Each test pins its language by injecting Accept-Language
            // through this lambda; the value is captured here.
            .layer(axum::middleware::from_fn(move |mut req: axum::extract::Request, next: axum::middleware::Next| {
                if !lang.is_empty() {
                    req.headers_mut().insert(
                        http::header::ACCEPT_LANGUAGE,
                        HeaderValue::from_static(lang),
                    );
                }
                async move { next.run(req).await }
            }))
    }

    use tower::ServiceExt;

    #[tokio::test]
    async fn opted_in_handler_produces_translated_body() {
        let payload = json!({
            "diagnostic": { "code": "auth.token.expired", "params": {} }
        });
        let app = app_with_locale(payload, true, "es");
        let resp = app
            .oneshot(HttpRequest::builder().uri("/d").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_value(resp).await;
        assert_eq!(
            v["diagnostic"]["message"], "Tu sesión ha expirado.",
            "translated message must be inserted; body = {v}"
        );
        assert_eq!(
            v["diagnostic"]["code"], "auth.token.expired",
            "original code is preserved",
        );
    }

    #[tokio::test]
    async fn opted_in_diagnostics_array_is_translated_element_wise() {
        let payload = json!({
            "diagnostics": [
                { "code": "auth.token.expired", "params": {} },
                { "code": "with.param", "params": { "name": { "string": "Ana" } } }
            ]
        });
        let app = app_with_locale(payload, true, "es");
        let resp = app
            .oneshot(HttpRequest::builder().uri("/d").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let v = body_value(resp).await;
        assert_eq!(v["diagnostics"][0]["message"], "Tu sesión ha expirado.");
        assert_eq!(v["diagnostics"][1]["message"], "Hola, Ana!");
    }

    #[tokio::test]
    async fn non_opted_in_handler_is_untouched() {
        let payload = json!({
            "diagnostic": { "code": "auth.token.expired", "params": {} }
        });
        let app = app_with_locale(payload.clone(), false, "es");
        let resp = app
            .oneshot(HttpRequest::builder().uri("/d").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let v = body_value(resp).await;
        assert!(
            v["diagnostic"].get("message").is_none(),
            "rewriter must NOT touch responses without DiagnosticBody; body = {v}"
        );
        assert_eq!(v["diagnostic"]["code"], "auth.token.expired");
    }

    #[tokio::test]
    async fn sse_response_is_untouched_even_when_opted_in() {
        // A handler emits an SSE-shaped response and (incorrectly,
        // but defensively tested) opts in. The rewriter must still
        // bail because Content-Type is text/event-stream.
        let bundle = bundle();
        let sse_handler = || async {
            let mut resp = Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "text/event-stream")
                .body(Body::from(
                    "data: {\"diagnostic\":{\"code\":\"auth.token.expired\",\"params\":{}}}\n\n",
                ))
                .unwrap();
            resp.extensions_mut().insert(DiagnosticBody::new());
            resp
        };
        let app = Router::new()
            .route("/sse", get(sse_handler))
            .layer(diagnostics_layer(bundle.clone()))
            .layer(crate::middleware::accept_language_layer(bundle));
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/sse")
                    .header(http::header::ACCEPT_LANGUAGE, "es")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            "text/event-stream"
        );
        let text = body_text(resp).await;
        // Body is verbatim — no "message" field was injected, no JSON
        // re-serialisation happened.
        assert!(
            text.contains("\"code\":\"auth.token.expired\""),
            "SSE body must be forwarded byte-for-byte; got = {text:?}",
        );
        assert!(
            !text.contains("Tu sesión"),
            "SSE body MUST NOT carry a translated message; got = {text:?}",
        );
    }

    #[tokio::test]
    async fn missing_translation_leaves_envelope_intact() {
        // Code that has no entry in the bundle — the envelope must
        // survive untouched (no `message` field, no rewrite).
        let payload = json!({
            "diagnostic": { "code": "nope.not.translated", "params": {} }
        });
        let app = app_with_locale(payload, true, "es");
        let resp = app
            .oneshot(HttpRequest::builder().uri("/d").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let v = body_value(resp).await;
        assert_eq!(v["diagnostic"]["code"], "nope.not.translated");
        assert!(
            v["diagnostic"].get("message").is_none(),
            "missing translation must NOT inject a message; body = {v}"
        );
    }

    #[tokio::test]
    async fn client_ignoring_content_language_still_sees_code_and_params() {
        // The rewriter is additive — code + params survive intact so
        // a consumer that translates client-side sees what it always
        // did. This is the "doesn't change behaviour for clients
        // that ignore Content-Language" guarantee.
        let payload = json!({
            "diagnostic": {
                "code": "with.param",
                "params": { "name": { "string": "Ana" } }
            }
        });
        let app = app_with_locale(payload, true, "es");
        let resp = app
            .oneshot(HttpRequest::builder().uri("/d").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let v = body_value(resp).await;
        assert_eq!(v["diagnostic"]["code"], "with.param");
        assert_eq!(
            v["diagnostic"]["params"]["name"]["string"], "Ana",
            "params survive verbatim",
        );
        assert_eq!(v["diagnostic"]["message"], "Hola, Ana!");
    }

    // ---- Pure-function tests --------------------------------------

    #[test]
    fn interpolate_substitutes_named_params() {
        let params = json!({ "name": { "string": "Ana" }, "n": { "i64": 3 } });
        assert_eq!(
            interpolate("Hi {name}, n={n}", Some(&params)),
            "Hi Ana, n=3"
        );
    }

    #[test]
    fn interpolate_leaves_unknown_placeholders_literal() {
        let params = json!({ "name": { "string": "Ana" } });
        assert_eq!(
            interpolate("Hi {name}, missing={who}", Some(&params)),
            "Hi Ana, missing={who}"
        );
    }

    #[test]
    fn interpolate_handles_no_params() {
        assert_eq!(interpolate("plain", None), "plain");
        // Unknown placeholders survive verbatim when no params are
        // supplied at all.
        assert_eq!(interpolate("hi {x}", None), "hi {x}");
    }

    #[test]
    fn interpolate_unterminated_brace_survives() {
        assert_eq!(interpolate("oops {unterminated", None), "oops {unterminated");
    }

    #[test]
    fn is_json_accepts_application_json_and_structured_suffix() {
        let mk = |ct: &'static str| {
            let mut r = Response::new(Body::empty());
            r.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(ct));
            r
        };
        assert!(is_json(&mk("application/json")));
        assert!(is_json(&mk("application/json; charset=utf-8")));
        assert!(is_json(&mk("application/problem+json")));
        assert!(!is_json(&mk("text/event-stream")));
        assert!(!is_json(&mk("text/plain")));
        let empty: Response<Body> = Response::new(Body::empty());
        assert!(!is_json(&empty));
    }

    #[test]
    fn is_streaming_flags_sse_and_chunked() {
        let mk = |hk: &'static str, hv: &'static str| {
            let mut r = Response::new(Body::empty());
            r.headers_mut().insert(
                http::HeaderName::from_static(hk),
                HeaderValue::from_static(hv),
            );
            r
        };
        assert!(is_streaming(&mk("content-type", "text/event-stream")));
        assert!(is_streaming(&mk("content-type", "text/event-stream; charset=utf-8")));
        assert!(is_streaming(&mk("transfer-encoding", "chunked")));
        assert!(is_streaming(&mk("transfer-encoding", "gzip, chunked")));
        assert!(!is_streaming(&mk("content-type", "application/json")));
    }
}
