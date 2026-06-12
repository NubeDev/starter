//! Extension-to-extension peer-call capability — shared wire types
//! (WS-18 Wave B).
//!
//! The `extension` capability lets an extension synchronously invoke another
//! extension's **provided** tool/node through the `extension.call` host method.
//! The call runs under the *caller's* identity (tenant + teams), never the
//! callee's — a callee cannot launder authority on the caller's behalf
//! (WS-14 §4.3). A target is reachable only when it appears in all three of:
//! the caller's `extension` grant `targets`, the caller's
//! `requires.extensions[].provides`, and the callee's `contributes.provides[]`.
//!
//! This crate is zero-runtime-logic (SCOPE R2); the dispatch (resolving the
//! callee's running child and forwarding the request) lives in the host
//! integration crate (`nexus-api`). The capability *handle* lives in
//! [`starter-ext-sdk::ctx::ExtensionCallHandle`].

use serde::{Deserialize, Serialize};

/// Wire payload for `extension.call` — a synchronous call into a peer
/// extension's provided tool/node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionCallRequest {
    /// The callee extension's id (e.g. `com.acme.geocode`).
    pub extension_id: String,
    /// The provided id to invoke — a tool id or node kind the callee lists in
    /// `contributes.provides[]`. Must be in the caller's grant `targets` as
    /// `"<extension_id>:<provided_id>"`.
    pub provided_id: String,
    /// The input payload, forwarded verbatim to the callee's tool/node body
    /// (validated against the provider's `input_schema` if declared).
    #[serde(default)]
    pub input: serde_json::Value,
}

/// Wire response for `extension.call`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionCallResponse {
    /// The callee's output, forwarded verbatim (validated against the
    /// provider's `output_schema` if declared).
    #[serde(default)]
    pub output: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_request_round_trip() {
        let r = ExtensionCallRequest {
            extension_id: "com.acme.geocode".into(),
            provided_id: "com.acme.geocode.lookup".into(),
            input: serde_json::json!({ "addr": "1 Main St" }),
        };
        let back: ExtensionCallRequest =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn call_response_round_trip() {
        let r = ExtensionCallResponse {
            output: serde_json::json!({ "lat": 1.0, "lon": 2.0 }),
        };
        let back: ExtensionCallResponse =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }
}
