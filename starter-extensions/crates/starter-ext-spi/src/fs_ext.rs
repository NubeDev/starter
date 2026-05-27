//! Wire types for the `fs.read` host method.
//!
//! `ctx.fs().read(path)` reads bytes from a host-side file. For
//! process-flavour callers the SDK marshals each call as a
//! JSON-RPC request; the supervisor's capability gate enforces the
//! `fs` category, and the host's installed [`HostMethodHandler`]
//! matches the path against the `Capability::Fs { paths }`
//! manifest grant before reading.
//!
//! Bytes are base64-encoded on the wire (`base64` standard
//! alphabet, no-pad). The SDK side decodes into `Vec<u8>` before
//! returning to the extension.

use serde::{Deserialize, Serialize};

/// Wire payload an extension sends on `fs.read`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsReadRequest {
    /// Path to read. The host matches it against the
    /// `Capability::Fs { paths }` allowlist. Path semantics
    /// (relative-to-bundle, absolute, glob) are host-defined —
    /// the wire keeps a single string and the host enforces.
    pub path: String,
}

/// Wire response for `fs.read`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsReadResponse {
    /// File contents as base64. Standard alphabet without
    /// padding (`base64::engine::general_purpose::STANDARD`).
    pub bytes_b64: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_read_request_round_trip() {
        let req = FsReadRequest {
            path: "config/thresholds.yaml".into(),
        };
        let j = serde_json::to_value(&req).unwrap();
        let back: FsReadRequest = serde_json::from_value(j).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn fs_read_response_round_trip() {
        let res = FsReadResponse {
            bytes_b64: "aGVsbG8=".into(),
        };
        let j = serde_json::to_value(&res).unwrap();
        let back: FsReadResponse = serde_json::from_value(j).unwrap();
        assert_eq!(back, res);
    }
}
