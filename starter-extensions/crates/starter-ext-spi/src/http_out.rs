//! Wire types for the `http.request` host method.
//!
//! `ctx.http_out().request(req)` issues an outbound HTTP call.
//! For process-flavour callers the SDK marshals each call as a
//! JSON-RPC request on the substrate's host-method channel; the
//! supervisor's capability gate enforces the `http_out` category,
//! and the host's installed [`HostMethodHandler`] checks the
//! request URL's authority against the
//! `Capability::HttpOut { authorities }` manifest grant before
//! issuing the call.
//!
//! Bodies are JSON-serialised (`serde_json::Value`) for v0.1 —
//! the kernel doesn't yet support streaming or binary uploads.
//! Future v2: a `body_b64` field for binary payloads.
//!
//! [`HostMethodHandler`]: ../../../starter-extensions/crates/starter-ext-supervisor/src/host_methods.rs

use serde::{Deserialize, Serialize};

/// Wire payload an extension sends on `http.request`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpRequest {
    /// HTTP method (`"GET"`, `"POST"`, …). Case-insensitive on the
    /// host side; unknown verbs are refused with
    /// `Error::Validation`.
    pub method: String,
    /// Fully-qualified URL. The host parses out the authority
    /// (`host[:port]`) and matches it against the manifest's
    /// `Capability::HttpOut { authorities }` allowlist.
    pub url: String,
    /// Headers as `[(name, value), …]`. Hop-by-hop headers
    /// (Connection, Keep-Alive, …) are dropped by the host.
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// Optional JSON body. When `None`, no body is sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// Wire response for `http.request`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpResponse {
    /// Status code (e.g. `200`, `404`).
    pub status: u16,
    /// Response headers, same shape as the request's.
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// Response body decoded as UTF-8 JSON if the
    /// `content-type` was `application/json`, else as a string.
    /// `null` for empty bodies.
    #[serde(default)]
    pub body: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_request_round_trip() {
        let req = HttpRequest {
            method: "POST".into(),
            url: "https://api.example.com/v1/things".into(),
            headers: vec![("content-type".into(), "application/json".into())],
            body: Some(serde_json::json!({"k": "v"})),
        };
        let j = serde_json::to_value(&req).unwrap();
        let back: HttpRequest = serde_json::from_value(j).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn http_request_minimal_round_trips() {
        // No headers, no body.
        let req = HttpRequest {
            method: "GET".into(),
            url: "https://api.example.com/v1/things".into(),
            headers: vec![],
            body: None,
        };
        let j = serde_json::to_value(&req).unwrap();
        let back: HttpRequest = serde_json::from_value(j).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn http_response_round_trip() {
        let res = HttpResponse {
            status: 200,
            headers: vec![("x-trace-id".into(), "abc".into())],
            body: serde_json::json!({"ok": true}),
        };
        let j = serde_json::to_value(&res).unwrap();
        let back: HttpResponse = serde_json::from_value(j).unwrap();
        assert_eq!(back, res);
    }
}
