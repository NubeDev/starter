//! Client half of the in-memory transport pair.
//!
//! Owns the outbound sender + inbound receiver. Frames are JSON-RPC
//! strings — the on-wire form, so tests exercise the same serialisation
//! the HTTP and stdio transports do (see
//! `docs/design/starter-changes/README.md`, Phase 2b U2).

use std::io;

use serde::Serialize;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::protocol::Response;

use super::Frame;

/// Client end of the paired in-memory transport. Send a JSON-RPC frame
/// with [`send_frame`](Self::send_frame) or one of the convenience
/// helpers; read the response with [`recv`](Self::recv).
pub struct InMemoryClient {
    tx: mpsc::Sender<Frame>,
    rx: mpsc::Receiver<Frame>,
}

impl InMemoryClient {
    pub(super) fn new(tx: mpsc::Sender<Frame>, rx: mpsc::Receiver<Frame>) -> Self {
        Self { tx, rx }
    }

    /// Send an already-serialised JSON-RPC frame. The server task picks
    /// it up off the channel and dispatches it. Returns an `io::Error`
    /// if the server has shut down.
    pub async fn send_frame(&self, frame: String) -> io::Result<()> {
        self.tx
            .send(frame)
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "server channel closed"))
    }

    /// Receive the next JSON-RPC frame from the server, parsed into a
    /// [`Response`]. Returns `None` when the server task has exited
    /// without producing further frames.
    pub async fn recv(&mut self) -> Option<Response> {
        let raw = self.rx.recv().await?;
        // The framing on this transport is "one channel item = one frame",
        // so parse failure here means the dispatch core produced
        // something malformed — surface a parse error rather than silently
        // dropping it. Tests assert on the response value, so a panic-on-
        // bad-json is loud-by-design.
        match serde_json::from_str::<RawResponse>(&raw) {
            Ok(rr) => Some(rr.into()),
            Err(e) => panic!("in-memory transport produced unparseable frame: {e}\nframe: {raw}"),
        }
    }

    /// Send a request built from method + params and await the matching
    /// response. The caller picks the JSON-RPC `id`.
    pub async fn request(&mut self, id: i64, method: &str, params: Value) -> io::Result<Response> {
        #[derive(Serialize)]
        struct Req<'a> {
            jsonrpc: &'static str,
            id: i64,
            method: &'a str,
            #[serde(skip_serializing_if = "Value::is_null")]
            params: Value,
        }
        let frame = serde_json::to_string(&Req {
            jsonrpc: "2.0",
            id,
            method,
            params,
        })
        .expect("Req is always serialisable");
        self.send_frame(frame).await?;
        self.recv()
            .await
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "server returned no frame"))
    }
}

/// Owned deserialisation target for [`Response`], whose public
/// `jsonrpc` field is `&'static str` (it is built fresh on the server).
/// Parsing back into an owned form keeps the wire shape symmetrical
/// without forcing the protocol module to learn about `Deserialize`.
#[derive(serde::Deserialize)]
struct RawResponse {
    #[serde(rename = "jsonrpc")]
    _jsonrpc: String,
    id: Value,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RawError>,
}

#[derive(serde::Deserialize)]
struct RawError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

impl From<RawResponse> for Response {
    fn from(r: RawResponse) -> Self {
        Self {
            jsonrpc: "2.0",
            id: r.id,
            result: r.result,
            error: r.error.map(|e| crate::protocol::RpcError {
                code: e.code,
                message: e.message,
                data: e.data,
            }),
        }
    }
}
