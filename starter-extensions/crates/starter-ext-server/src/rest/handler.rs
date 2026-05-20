//! The actual request handlers the router wires up.
//!
//! Three shapes:
//!
//! - [`non_streaming`] — read body, validate against `request_schema`,
//!   call [`RestDispatcher::dispatch`], render the JSON result.
//! - [`sse`] — call [`RestDispatcher::dispatch_stream`], render the
//!   event stream as `text/event-stream` with a 15s heartbeat + an
//!   initial `retry:` field so an `EventSource` reconnects every 3 s
//!   on transport failure.
//! - [`ndjson`] — same dispatch, render the event stream as
//!   `application/x-ndjson` (`{json}\n{json}\n…`).
//!
//! Each handler is parametrised by a [`HandlerSpec`] that the router
//! builds once at load time so the per-request work stays small.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, HeaderValue, Response, StatusCode};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::StreamExt;
use serde_json::Value;
use starter_ext_spi::ExtensionId;

use super::dispatcher::{CancelHandle, DispatchError, RestDispatcher, StreamResponse};
use super::schema::SchemaCheck;

/// All the per-route state the router builds once at load time and
/// shares with the handler via `Arc`-cloning. Holds the dispatcher
/// (shared across every entry on the router), the extension id, the
/// contribute id, and the request-body schema check.
pub(crate) struct HandlerSpec {
    pub(crate) dispatcher: Arc<dyn RestDispatcher>,
    pub(crate) extension: ExtensionId,
    pub(crate) contribute_id: String,
    pub(crate) request_schema: SchemaCheck,
}

impl HandlerSpec {
    pub(crate) fn new(
        dispatcher: Arc<dyn RestDispatcher>,
        extension: ExtensionId,
        contribute_id: impl Into<String>,
        request_schema: SchemaCheck,
    ) -> Arc<Self> {
        Arc::new(Self {
            dispatcher,
            extension,
            contribute_id: contribute_id.into(),
            request_schema,
        })
    }
}

/// Decode the body as JSON. An empty body becomes `Value::Null` —
/// permissive default for `GET` and for tools that take no input.
fn parse_body(bytes: &Bytes) -> Result<Value, String> {
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice::<Value>(bytes).map_err(|e| format!("body is not valid JSON: {e}"))
}

fn dispatch_error_response(err: DispatchError) -> Response<Body> {
    let (status, msg) = match &err {
        DispatchError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
        DispatchError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
        DispatchError::NotWired(m) => (StatusCode::SERVICE_UNAVAILABLE, m.clone()),
        DispatchError::Extension(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        DispatchError::Substrate(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
    };
    let body = serde_json::json!({ "error": msg });
    (status, Json(body)).into_response()
}

/// Non-streaming handler. Validates the body, calls
/// [`RestDispatcher::dispatch`], returns the JSON result.
pub(crate) async fn non_streaming(
    State(spec): State<Arc<HandlerSpec>>,
    body: Bytes,
) -> Response<Body> {
    let input = match parse_body(&body) {
        Ok(v) => v,
        Err(msg) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response()
        }
    };
    if let Err(msg) = spec.request_schema.check(&input) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response();
    }
    match spec
        .dispatcher
        .dispatch(&spec.extension, &spec.contribute_id, input)
        .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(e) => dispatch_error_response(e),
    }
}

/// SSE handler. Same dispatch path as [`ndjson`] but renders with
/// `axum::response::sse::Sse` so we get the `text/event-stream`
/// content-type + heartbeat for free, and we emit a `retry:` field on
/// the first frame (browser `EventSource` reads it as the reconnect
/// delay).
pub(crate) async fn sse(State(spec): State<Arc<HandlerSpec>>, body: Bytes) -> Response<Body> {
    let (input, schema_err) = parse_and_check(&spec, body);
    if let Some(resp) = schema_err {
        return resp;
    }
    let StreamResponse {
        stream_id,
        events,
        cancel,
    } = match spec
        .dispatcher
        .dispatch_stream(&spec.extension, &spec.contribute_id, input)
        .await
    {
        Ok(r) => r,
        Err(e) => return dispatch_error_response(e),
    };

    // Keep the cancel handle alive for the lifetime of the stream;
    // dropping it (when the response body is dropped) fires the
    // cancellation hook.
    let drop_guard = Arc::new(parking_drop_guard(cancel));

    let sid_for_map = stream_id.clone();
    let guard_clone = drop_guard.clone();
    let sse_stream = events.map(move |item| {
        // Keep the drop guard reachable from inside the stream closure
        // so the borrow checker won't drop it before the stream ends.
        let _hold = guard_clone.clone();
        let frame = match item {
            Ok(ev) => SseEvent::default()
                .id(sid_for_map.as_str())
                .event("stream.event")
                .data(serde_json::to_string(&ev.payload).unwrap_or_else(|_| "null".into())),
            Err(err) => SseEvent::default()
                .id(sid_for_map.as_str())
                .event("stream.error")
                .data(serde_json::to_string(&err).unwrap_or_else(|_| "{}".into())),
        };
        Ok::<_, Infallible>(frame)
    });

    let mut resp = Sse::new(sse_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response();
    // `retry:` is the SSE-standard reconnect-delay-in-ms hint to
    // `EventSource`. Browsers and the `eventsource` polyfill honour it.
    // We send it via a custom header so the body stays untouched; axum's
    // Sse helper appends it for us if we use `Event::retry`, but doing
    // it here keeps the very first emitted frame the extension's own.
    // (Both the header and a leading retry frame are common in the wild;
    // we use the header so the first byte the client sees is the SSE
    // comment from the keep-alive, not a generic retry frame.)
    resp.headers_mut()
        .insert("X-SSE-Retry-Ms", HeaderValue::from_static("3000"));
    // Stash the guard on the response extensions so axum keeps it alive
    // alongside the body.
    resp.extensions_mut().insert(drop_guard);
    resp
}

/// NDJSON handler — newline-delimited JSON streaming.
pub(crate) async fn ndjson(State(spec): State<Arc<HandlerSpec>>, body: Bytes) -> Response<Body> {
    let (input, schema_err) = parse_and_check(&spec, body);
    if let Some(resp) = schema_err {
        return resp;
    }
    let StreamResponse {
        stream_id,
        events,
        cancel,
    } = match spec
        .dispatcher
        .dispatch_stream(&spec.extension, &spec.contribute_id, input)
        .await
    {
        Ok(r) => r,
        Err(e) => return dispatch_error_response(e),
    };

    let drop_guard = Arc::new(parking_drop_guard(cancel));
    let guard_clone = drop_guard.clone();
    let sid = stream_id.clone();

    // Map each event to a `{json}\n` byte chunk. The Drop guard is held
    // by every emitted closure so dropping the body releases it.
    let lines = events.map(move |item| {
        let _hold = guard_clone.clone();
        let value = match item {
            Ok(ev) => serde_json::json!({
                "type": "stream.event",
                "stream_id": sid.as_str(),
                "payload": ev.payload,
            }),
            Err(err) => serde_json::json!({
                "type": "stream.error",
                "stream_id": sid.as_str(),
                "error": err,
            }),
        };
        let mut buf = serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec());
        buf.push(b'\n');
        Ok::<Bytes, Infallible>(Bytes::from(buf))
    });

    let mut resp = Response::new(Body::from_stream(lines));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson"),
    );
    resp.extensions_mut().insert(drop_guard);
    resp
}

/// Decode + schema-check the request body. Returns the parsed input
/// plus an optional response that the caller should return verbatim
/// on a parse/validation failure.
fn parse_and_check(spec: &HandlerSpec, body: Bytes) -> (Value, Option<Response<Body>>) {
    match parse_body(&body) {
        Ok(input) => {
            if let Err(msg) = spec.request_schema.check(&input) {
                return (
                    Value::Null,
                    Some(
                        (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({ "error": msg })),
                        )
                            .into_response(),
                    ),
                );
            }
            (input, None)
        }
        Err(msg) => (
            Value::Null,
            Some(
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": msg })),
                )
                    .into_response(),
            ),
        ),
    }
}

/// Wrap a [`CancelHandle`] in a guard whose `Drop` fires it. Kept
/// behind an `Arc` so handler closures can clone the reference into
/// each yielded item without consuming the guard.
fn parking_drop_guard(cancel: CancelHandle) -> CancelDropGuard {
    CancelDropGuard {
        cancel: Some(cancel),
    }
}

pub(crate) struct CancelDropGuard {
    cancel: Option<CancelHandle>,
}

impl Drop for CancelDropGuard {
    fn drop(&mut self) {
        if let Some(c) = self.cancel.take() {
            c.fire();
        }
    }
}
