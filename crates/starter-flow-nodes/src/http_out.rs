//! `http-out` — outbound HTTP request node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "Relationship to
//! existing crates" (the `starter-flow-nodes` row lists `http-out`
//! alongside the other built-ins) and scheduled in § "Phase 5 —
//! Remaining built-in node kinds". Issues exactly one outbound HTTP
//! request shaped by its input slots and emits the response on its
//! output slots. Retry / backoff / circuit-break stay engine-side
//! per § "R3 — The engine is a reader of policies, never an owner";
//! the body only carries a per-request timeout as a safety net so a
//! pathological remote never wedges the run alone.
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** Plain [`NodeBehavior`] impl.
//! - **R2 — One write chokepoint.** The body returns the response
//!   slots; the propagator funnels through `GraphStore::write_slot`.
//! - **R3 — Engine reads policies.** Retry / timeout *policy* is
//!   engine-owned; the body only takes a single safety-net
//!   `timeout_ms` per invocation.
//! - **R5 — Stateless behaviours.** The [`reqwest::Client`] is built
//!   once at construction time and reused across invocations; no
//!   per-invocation mutation.
//! - **R10 — Reverse-DNS ids.** [`KIND_ID`] verbatim under
//!   `starter.flow.*`.
//! - **R12 observability.** `http_out.invoke` tracing span records
//!   `(node_id, run_id, method, url_host, status, cancel_observed)`.
//! - **R13 cancellation.** The request future is `tokio::select!`ed
//!   against `ctx.cancel.cancelled()`; a cancelled run aborts the
//!   request immediately and surfaces [`NodeError::Cancelled`].

use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Client, Method};
use schemars::{schema::RootSchema, JsonSchema};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use thiserror::Error;
use tracing::Instrument;

use starter_flow_spi::node::{
    anyhow_compat, KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace.
pub const KIND_ID: &str = "starter.flow.http-out";

/// Static metadata for the catalog / discovery surface.
pub static DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.http-out.label",
        "starter.flow.node.http-out.summary",
        "starter.flow.node.http-out.help",
    );

/// Mandatory input slot carrying the request URL. Must be a
/// [`SlotValue::String`] parseable as `http://` or `https://`.
pub const URL_SLOT: &str = "url";

/// Optional input slot naming the HTTP method. Case-insensitive.
/// Defaults to `GET` when absent. Supported: `GET`, `POST`, `PUT`,
/// `PATCH`, `DELETE`, `HEAD`.
pub const METHOD_SLOT: &str = "method";

/// Optional input slot carrying the request body. A
/// [`SlotValue::String`] is sent verbatim; [`SlotValue::Json`] is
/// serialised and sent with `Content-Type: application/json` (unless
/// the caller already supplied that header); [`SlotValue::Bytes`] is
/// sent as `application/octet-stream`.
pub const BODY_SLOT: &str = "body";

/// Optional input slot carrying request headers as a
/// [`SlotValue::Json`] object of `string → string`.
pub const HEADERS_SLOT: &str = "headers";

/// Optional input slot carrying a per-request timeout (milliseconds).
/// Defaults to [`DEFAULT_TIMEOUT_MS`] when absent.
pub const TIMEOUT_MS_SLOT: &str = "timeout_ms";

/// Output slot carrying the response status code as
/// [`SlotValue::Int`].
pub const STATUS_SLOT: &str = "status";

/// Output slot carrying response headers as a [`SlotValue::Json`]
/// object of `string → string`. Multi-valued headers are joined with
/// `, `.
pub const RESPONSE_HEADERS_SLOT: &str = "response_headers";

/// Output slot carrying the response body. Parses the body as JSON
/// when the response advertises `Content-Type: application/json`;
/// otherwise emits the body as a [`SlotValue::String`].
pub const RESPONSE_BODY_SLOT: &str = "response_body";

/// Safety-net request timeout when the input slot is absent.
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Hard ceiling on the configurable timeout. Held to ten minutes so a
/// typo can't pin a flow worker indefinitely.
pub const MAX_TIMEOUT_MS: u64 = 10 * 60 * 1000;

/// Publish-time configuration carried on an `http-out` node's
/// `settings:` field in a flow body. Per
/// [`DOCS/flow/scope/settings.md`](../../../DOCS/flow/scope/settings.md)
/// Phase S-4: declares the typed schema editor surfaces validate
/// drafts against.
///
/// Distinct from the *runtime* input slots ([`URL_SLOT`],
/// [`METHOD_SLOT`], …) — those receive values from upstream nodes at
/// invoke time. The settings here are the *publish-time* defaults a
/// flow author writes into the body; once `TopologyResolver::resolve`
/// lands (`DOCS/flow/scope/hot-reload.md` HR5) it will seed each
/// field into the matching config slot. All fields are optional so a
/// fully-dynamic `http-out` (every slot driven by an upstream link)
/// validates with an empty settings object.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HttpOutSettings {
    /// Default request URL. Must be an `http://` or `https://` URL
    /// when present.
    #[serde(default)]
    pub url: Option<String>,

    /// Default HTTP method. Case-insensitive at runtime; the schema
    /// constrains the publish-time value to the supported set.
    #[serde(default)]
    pub method: Option<HttpMethod>,

    /// Default request headers as a string→string map.
    #[serde(default)]
    pub headers: Option<BTreeMap<String, String>>,

    /// Default request body as a UTF-8 string. JSON bodies are
    /// modelled as a string at publish time so the schema stays
    /// expressible in plain JSON Schema; runtime callers wanting a
    /// structured body still link a [`BODY_SLOT`] from an upstream
    /// node that yields [`SlotValue::Json`].
    #[serde(default)]
    pub body: Option<String>,

    /// Default per-request timeout in milliseconds. Must satisfy
    /// `1 ..= MAX_TIMEOUT_MS` when present. Absent uses
    /// [`DEFAULT_TIMEOUT_MS`].
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// HTTP method values an `http-out` node accepts at publish time.
/// The runtime path is case-insensitive (see [`parse_method`]); this
/// enum is the canonical lower-case-friendly enumeration the schema
/// surfaces to editors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    /// HTTP `GET`.
    Get,
    /// HTTP `POST`.
    Post,
    /// HTTP `PUT`.
    Put,
    /// HTTP `PATCH`.
    Patch,
    /// HTTP `DELETE`.
    Delete,
    /// HTTP `HEAD`.
    Head,
}

/// Derived JSON Schema for [`HttpOutSettings`]. Returned by reference
/// from [`HttpOut::config_schema`]; built once per process via
/// [`LazyLock`].
pub static HTTP_OUT_SETTINGS_SCHEMA: LazyLock<RootSchema> =
    LazyLock::new(|| schemars::schema_for!(HttpOutSettings));

/// Typed errors surfaced by [`HttpOut::invoke`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum HttpOutError {
    /// The input did not carry a [`URL_SLOT`] entry.
    #[error("http-out input missing `{URL_SLOT}` slot")]
    MissingUrl,

    /// [`URL_SLOT`] was present but not a [`SlotValue::String`].
    #[error("http-out `{URL_SLOT}` must be SlotValue::String")]
    InvalidUrlType,

    /// [`METHOD_SLOT`] was present but not a recognised method.
    #[error("http-out unsupported method: `{0}` (expected GET|POST|PUT|PATCH|DELETE|HEAD)")]
    InvalidMethod(String),

    /// [`METHOD_SLOT`] was present but not a [`SlotValue::String`].
    #[error("http-out `{METHOD_SLOT}` must be SlotValue::String")]
    InvalidMethodType,

    /// [`HEADERS_SLOT`] was present but not a JSON object of strings.
    #[error("http-out `{HEADERS_SLOT}` must be SlotValue::Json(object) with string values")]
    InvalidHeaders,

    /// [`TIMEOUT_MS_SLOT`] was negative, zero, or above the ceiling.
    #[error(
        "http-out `{TIMEOUT_MS_SLOT}` out of range: {0} \
         (must be 1..={MAX_TIMEOUT_MS})"
    )]
    TimeoutOutOfRange(i64),

    /// [`TIMEOUT_MS_SLOT`] was present but not a [`SlotValue::Int`].
    #[error("http-out `{TIMEOUT_MS_SLOT}` must be SlotValue::Int milliseconds")]
    InvalidTimeoutType,
}

impl HttpOutError {
    fn into_node_error(self) -> NodeError {
        NodeError::Other(anyhow_compat::Error(Box::new(self)))
    }
}

/// `http-out` node-kind behaviour. Stateless (R5) — the shared
/// [`reqwest::Client`] is immutable for the lifetime of the body.
pub struct HttpOut {
    kind: KindId,
    client: Client,
}

impl Default for HttpOut {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpOut {
    /// Construct an [`HttpOut`] body with a default
    /// [`reqwest::Client`].
    ///
    /// Panics only on a reqwest TLS configuration failure (the host
    /// is misconfigured) — there's no production path that hits it.
    pub fn new() -> Self {
        let client = Client::builder()
            .build()
            .expect("default reqwest client builds on supported platforms");
        Self::with_client(client)
    }

    /// Construct an [`HttpOut`] body around a caller-supplied client.
    /// Used by integration tests that want a client preconfigured for
    /// a test harness (e.g. with a `Resolver` pointing at a local
    /// listener, or `danger_accept_invalid_certs(true)`).
    pub fn with_client(client: Client) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
            client,
        }
    }
}

fn parse_method(s: &str) -> Result<Method, HttpOutError> {
    match s.to_ascii_uppercase().as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        "PUT" => Ok(Method::PUT),
        "PATCH" => Ok(Method::PATCH),
        "DELETE" => Ok(Method::DELETE),
        "HEAD" => Ok(Method::HEAD),
        other => Err(HttpOutError::InvalidMethod(other.to_owned())),
    }
}

#[async_trait]
impl NodeBehavior for HttpOut {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    fn trigger_slots(&self) -> &'static [&'static str] {
        &[BODY_SLOT]
    }

    fn read_slots(&self) -> &'static [&'static str] {
        &[URL_SLOT, METHOD_SLOT, HEADERS_SLOT, TIMEOUT_MS_SLOT]
    }

    fn config_schema(&self) -> &'static RootSchema {
        &HTTP_OUT_SETTINGS_SCHEMA
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        // URL ----------------------------------------------------------
        let url = match input.remove(URL_SLOT) {
            None => return Err(HttpOutError::MissingUrl.into_node_error()),
            Some(SlotValue::String(s)) => s,
            Some(_) => return Err(HttpOutError::InvalidUrlType.into_node_error()),
        };

        // Method -------------------------------------------------------
        let method = match input.remove(METHOD_SLOT) {
            None => Method::GET,
            Some(SlotValue::String(s)) => {
                parse_method(&s).map_err(HttpOutError::into_node_error)?
            }
            Some(_) => return Err(HttpOutError::InvalidMethodType.into_node_error()),
        };

        // Headers ------------------------------------------------------
        let mut headers: BTreeMap<String, String> = BTreeMap::new();
        if let Some(v) = input.remove(HEADERS_SLOT) {
            let obj = match v {
                SlotValue::Json(JsonValue::Object(m)) => m,
                _ => return Err(HttpOutError::InvalidHeaders.into_node_error()),
            };
            for (k, val) in obj {
                match val {
                    JsonValue::String(s) => {
                        headers.insert(k, s);
                    }
                    _ => return Err(HttpOutError::InvalidHeaders.into_node_error()),
                }
            }
        }

        // Timeout ------------------------------------------------------
        let timeout_ms = match input.remove(TIMEOUT_MS_SLOT) {
            None => DEFAULT_TIMEOUT_MS,
            Some(SlotValue::Int(n)) => {
                if n <= 0 || (n as u64) > MAX_TIMEOUT_MS {
                    return Err(HttpOutError::TimeoutOutOfRange(n).into_node_error());
                }
                n as u64
            }
            Some(_) => return Err(HttpOutError::InvalidTimeoutType.into_node_error()),
        };

        // Body ---------------------------------------------------------
        let body_slot = input.remove(BODY_SLOT);

        let url_host = reqwest::Url::parse(&url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_else(|| "<unparseable>".into());

        let span = tracing::info_span!(
            "http_out.invoke",
            node_id = %ctx.node,
            run_id = %ctx.run,
            method = %method,
            url_host = %url_host,
            status = tracing::field::Empty,
            cancel_observed = tracing::field::Empty,
        );
        // `Instrument` (not `span.enter()`) — the body contains
        // many `.await` points (request send, body read). A span
        // guard across `.await` corrupts the thread-local span
        // stack if the future migrates between tokio workers,
        // later panicking `tracing-subscriber` on an unrelated
        // emit.
        let span_for_record = span.clone();
        async move {
            // Build the request.
            let mut req = self
                .client
                .request(method, &url)
                .timeout(Duration::from_millis(timeout_ms));
            for (k, v) in &headers {
                req = req.header(k, v);
            }
            let json_ct = headers
                .keys()
                .any(|k| k.eq_ignore_ascii_case("content-type"));
            if let Some(b) = body_slot {
                req = match b {
                    SlotValue::String(s) => req.body(s),
                    SlotValue::Bytes(b) => {
                        if !json_ct {
                            req = req.header("content-type", "application/octet-stream");
                        }
                        req.body(b)
                    }
                    SlotValue::Json(j) => {
                        if json_ct {
                            // Caller already set Content-Type — send the
                            // serialised body but don't re-set the header.
                            req.body(j.to_string())
                        } else {
                            req.json(&j)
                        }
                    }
                    other => req.body(format!("{other:?}")),
                };
            }

            // Issue the request, racing against the cancel token.
            let response = tokio::select! {
                r = req.send() => r,
                () = ctx.cancel.cancelled() => {
                    span_for_record.record("cancel_observed", true);
                    return Err(NodeError::Cancelled);
                }
            };
            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    span_for_record.record("cancel_observed", false);
                    return Err(NodeError::Backend(format!("http-out request failed: {e}")));
                }
            };

            let status = response.status().as_u16();
            span_for_record.record("status", status);

            // Snapshot headers before consuming the body.
            let mut resp_headers = serde_json::Map::new();
            for (name, value) in response.headers() {
                let key = name.as_str().to_owned();
                let val = value.to_str().unwrap_or("<binary>").to_owned();
                // Multi-valued: join with ", " (HTTP-spec idiomatic for
                // most headers).
                resp_headers
                    .entry(key)
                    .and_modify(|e| {
                        if let JsonValue::String(prev) = e {
                            *prev = format!("{prev}, {val}");
                        }
                    })
                    .or_insert_with(|| JsonValue::String(val.clone()));
            }
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();

            // Body: parse as JSON when the response so advertises.
            let body_slot_value = if content_type.starts_with("application/json") {
                let bytes = tokio::select! {
                    b = response.bytes() => b,
                    () = ctx.cancel.cancelled() => {
                        span_for_record.record("cancel_observed", true);
                        return Err(NodeError::Cancelled);
                    }
                };
                let bytes = bytes
                    .map_err(|e| NodeError::Backend(format!("http-out body read failed: {e}")))?;
                match serde_json::from_slice::<JsonValue>(&bytes) {
                    Ok(j) => SlotValue::Json(j),
                    Err(_) => SlotValue::String(String::from_utf8_lossy(&bytes).into_owned()),
                }
            } else {
                let text = tokio::select! {
                    t = response.text() => t,
                    () = ctx.cancel.cancelled() => {
                        span_for_record.record("cancel_observed", true);
                        return Err(NodeError::Cancelled);
                    }
                };
                SlotValue::String(
                    text.map_err(|e| {
                        NodeError::Backend(format!("http-out body read failed: {e}"))
                    })?,
                )
            };

            span_for_record.record("cancel_observed", false);

            let mut out = SlotMap::new();
            out.insert(STATUS_SLOT.to_owned(), SlotValue::Int(status as i64));
            out.insert(
                RESPONSE_HEADERS_SLOT.to_owned(),
                SlotValue::Json(JsonValue::Object(resp_headers)),
            );
            out.insert(RESPONSE_BODY_SLOT.to_owned(), body_slot_value);
            Ok(out)
        }
        .instrument(span)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use starter_flow_spi::node::NodeId;
    use starter_flow_spi::Cancel;

    struct NoCancel;
    impl Cancel for NoCancel {
        fn is_cancelled(&self) -> bool {
            false
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(std::future::pending())
        }
    }

    struct FlagCancel {
        flag: Arc<AtomicBool>,
    }
    impl Cancel for FlagCancel {
        fn is_cancelled(&self) -> bool {
            self.flag.load(Ordering::SeqCst)
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            let flag = self.flag.clone();
            Box::pin(async move {
                while !flag.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            })
        }
    }

    fn make_ctx<'a>(node: &'a NodeId, cancel: &'a dyn Cancel) -> NodeCtx<'a> {
        NodeCtx::new(
            starter_flow_spi::flow::RunId::new(),
            node,
            cancel,
            starter_flow_spi::skill::SkillSelection::NONE,
            &starter_flow_spi::state::NOOP_NODE_STATE_STORE,
        )
    }

    /// Minimal one-shot HTTP/1.1 responder. Binds to localhost,
    /// accepts a single connection, reads the request bytes up to
    /// the body boundary, writes `response_bytes`, then closes.
    ///
    /// Returns `(bound_url, request_capture_handle)`.
    async fn one_shot_server(
        response_bytes: Vec<u8>,
    ) -> (String, tokio::task::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let url = format!("http://{addr}/echo");
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = Vec::with_capacity(4096);
            let mut tmp = [0u8; 1024];
            loop {
                let n = socket.read(&mut tmp).await.expect("read");
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                if let Some(idx) = find_header_end(&buf) {
                    let content_length = parse_content_length(&buf[..idx]);
                    let body_have = buf.len() - (idx + 4);
                    if body_have >= content_length {
                        break;
                    }
                }
            }
            socket.write_all(&response_bytes).await.expect("write");
            socket.flush().await.expect("flush");
            buf
        });
        (url, handle)
    }

    /// Compose an HTTP/1.1 response with a `Content-Type` header and
    /// an exact `Content-Length`. Returns the wire bytes.
    fn build_response(status_line: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
        let mut out = format!(
            "HTTP/1.1 {status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        out.extend_from_slice(body);
        out
    }

    fn find_header_end(buf: &[u8]) -> Option<usize> {
        buf.windows(4).position(|w| w == b"\r\n\r\n")
    }

    fn parse_content_length(headers: &[u8]) -> usize {
        let s = String::from_utf8_lossy(headers);
        for line in s.split("\r\n") {
            if let Some(v) = line
                .to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(str::trim)
                .and_then(|s| s.parse().ok())
            {
                return v;
            }
        }
        0
    }

    fn input_with(url: &str) -> SlotMap {
        let mut m = SlotMap::new();
        m.insert(URL_SLOT.to_owned(), SlotValue::String(url.to_owned()));
        m
    }

    #[tokio::test]
    async fn get_returns_status_and_text_body() {
        let response = build_response("200 OK", "text/plain", b"hello");
        let (url, server) = one_shot_server(response).await;

        let body = HttpOut::new();
        let node = NodeId::new("test.http-out").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let out = body.invoke(ctx, input_with(&url)).await.expect("invoke");

        assert!(matches!(out.get(STATUS_SLOT), Some(SlotValue::Int(200))));
        assert!(
            matches!(out.get(RESPONSE_BODY_SLOT), Some(SlotValue::String(s)) if s == "hello"),
            "unexpected body: {:?}",
            out.get(RESPONSE_BODY_SLOT)
        );
        let captured = server.await.expect("server task");
        assert!(captured.starts_with(b"GET /echo HTTP/1.1\r\n"));
    }

    #[tokio::test]
    async fn json_response_parses_to_json_slot() {
        let response = build_response("201 Created", "application/json", b"{\"ok\":true,\"n\":1}");
        let (url, _server) = one_shot_server(response).await;

        let body = HttpOut::new();
        let node = NodeId::new("test.http-out").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let out = body.invoke(ctx, input_with(&url)).await.expect("invoke");

        assert!(matches!(out.get(STATUS_SLOT), Some(SlotValue::Int(201))));
        match out.get(RESPONSE_BODY_SLOT) {
            Some(SlotValue::Json(j)) => {
                assert_eq!(j.get("ok"), Some(&JsonValue::Bool(true)));
                assert_eq!(j.get("n"), Some(&JsonValue::from(1_i64)));
            }
            other => panic!("expected Json response body, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn post_sends_json_body_with_content_type() {
        let response = build_response("200 OK", "text/plain", b"ok");
        let (url, server) = one_shot_server(response).await;

        let body = HttpOut::new();
        let node = NodeId::new("test.http-out").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);

        let mut input = input_with(&url);
        input.insert(METHOD_SLOT.to_owned(), SlotValue::String("POST".into()));
        input.insert(
            BODY_SLOT.to_owned(),
            SlotValue::Json(serde_json::json!({"hello":"world"})),
        );
        let _ = body.invoke(ctx, input).await.expect("invoke");

        let captured = server.await.expect("server task");
        let captured_str = String::from_utf8_lossy(&captured);
        assert!(
            captured_str.starts_with("POST /echo HTTP/1.1\r\n"),
            "{captured_str}"
        );
        assert!(
            captured_str
                .to_ascii_lowercase()
                .contains("content-type: application/json"),
            "{captured_str}"
        );
        assert!(
            captured_str.contains("\"hello\":\"world\""),
            "{captured_str}"
        );
    }

    #[tokio::test]
    async fn missing_url_is_error() {
        let body = HttpOut::new();
        let node = NodeId::new("test.http-out").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let err = body
            .invoke(ctx, SlotMap::new())
            .await
            .expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains(URL_SLOT), "{msg}");
    }

    #[tokio::test]
    async fn unsupported_method_is_error() {
        let body = HttpOut::new();
        let node = NodeId::new("test.http-out").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = input_with("http://127.0.0.1:1/nope");
        input.insert(METHOD_SLOT.to_owned(), SlotValue::String("BOGUS".into()));
        let err = body.invoke(ctx, input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains("unsupported method"), "{msg}");
    }

    #[tokio::test]
    async fn cancel_aborts_request() {
        // Bind a listener but never write a response so the request
        // would hang forever; cancel resolves it.
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let url = format!("http://{addr}/hang");
        tokio::spawn(async move {
            let (_socket, _) = listener.accept().await.expect("accept");
            // Hold the socket open without writing.
            tokio::time::sleep(Duration::from_secs(30)).await;
        });

        let flag = Arc::new(AtomicBool::new(false));
        let cancel = FlagCancel { flag: flag.clone() };

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            flag.store(true, Ordering::SeqCst);
        });

        let body = HttpOut::new();
        let node = NodeId::new("test.http-out").unwrap();
        let ctx = make_ctx(&node, &cancel);

        let started = std::time::Instant::now();
        let err = body
            .invoke(ctx, input_with(&url))
            .await
            .expect_err("must cancel");
        assert!(matches!(err, NodeError::Cancelled), "{err}");
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn invalid_headers_shape_is_error() {
        let body = HttpOut::new();
        let node = NodeId::new("test.http-out").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = input_with("http://127.0.0.1:1/nope");
        // Number value where a string is required.
        input.insert(
            HEADERS_SLOT.to_owned(),
            SlotValue::Json(serde_json::json!({"x": 1})),
        );
        let err = body.invoke(ctx, input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains(HEADERS_SLOT), "{msg}");
    }

    #[tokio::test]
    async fn negative_timeout_is_error() {
        let body = HttpOut::new();
        let node = NodeId::new("test.http-out").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = input_with("http://127.0.0.1:1/nope");
        input.insert(TIMEOUT_MS_SLOT.to_owned(), SlotValue::Int(-1));
        let err = body.invoke(ctx, input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains("out of range"), "{msg}");
    }
}
