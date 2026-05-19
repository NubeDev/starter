//! Init handshake with manifest content-hash verification (SCOPE R3).
//!
//! Per **R3**:
//!
//! > For process flavour, the child binary and the host load the same
//! > `block.yaml`. The init handshake includes the manifest's content
//! > hash; mismatch (child built against a different manifest than the
//! > bundle ships) is refused by the supervisor with a clear deploy-time
//! > error.
//!
//! The wire shape is one request and one response, framed by
//! [`starter_jsonrpc_stdio`] like every other message on this stream:
//!
//! ```text
//! → host:   {"jsonrpc":"2.0","id":0,"method":"init","params":{
//!               "manifest_hash":"<sha256 hex>",
//!               "config": { … },
//!               "host_version":"0.1.0"
//!           }}
//! ← child:  {"jsonrpc":"2.0","id":0,"result":{
//!               "manifest_hash":"<sha256 hex>",
//!               "ready": true
//!           }}
//! ```
//!
//! The hash is computed over the raw bytes of `block.yaml` as they exist
//! in the bundle directory the supervisor was handed. The child re-hashes
//! the manifest it was *built against* (via `ExtensionMeta::manifest_yaml()`,
//! produced by `#[derive(Extension)]`) and echoes the digest back. The
//! supervisor refuses to transition to `Running` unless the two digests
//! match exactly. The intent is not cryptographic — a deployer that
//! tampered with the bundle could also recompute the hash — but to catch
//! the routine "you redeployed the bundle without recompiling the child"
//! failure mode where the schema-version handshake (`v: 1`) cannot.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// `init` request params. Sent host → child as the first frame after
/// spawn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitHandshake {
    /// SHA-256 of the bundle's `block.yaml`, hex-encoded.
    pub manifest_hash: String,
    /// Operator-supplied configuration from the manifest's `config:` field.
    /// Opaque JSON — validated against the bundle's `config_schema:` one
    /// layer up.
    #[serde(default)]
    pub config: serde_json::Value,
    /// Crate version of `starter-ext-host` running the supervisor.
    /// Surfaced so a child built against an older SDK can refuse to start
    /// when needed; the v0.1 SDK simply echoes it back.
    pub host_version: String,
}

/// `init` response payload (child → host).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitReady {
    /// The hash the child computed against the manifest it was built
    /// against. Must equal [`InitHandshake::manifest_hash`].
    pub manifest_hash: String,
    /// The child confirms `on_init` returned `Ok(())`. `false` is legal
    /// and surfaces a `Crashed` lifecycle transition without restarting
    /// through a panic.
    pub ready: bool,
    /// Optional human-readable reason when `ready` is `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// SHA-256 of a manifest's raw bytes, hex-encoded. Matches what the
/// `init` handshake exchanges. Pure function — no I/O.
pub fn manifest_hash(yaml_bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(yaml_bytes);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        let a = manifest_hash(b"v: 1\nid: com.acme.x\n");
        let b = manifest_hash(b"v: 1\nid: com.acme.x\n");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn hash_changes_on_byte_change() {
        let a = manifest_hash(b"id: com.acme.x\n");
        let b = manifest_hash(b"id: com.acme.y\n");
        assert_ne!(a, b);
    }

    #[test]
    fn handshake_round_trips_json() {
        let req = InitHandshake {
            manifest_hash: "ab".into(),
            config: serde_json::json!({ "key": "value" }),
            host_version: "0.1.0".into(),
        };
        let j = serde_json::to_string(&req).unwrap();
        let back: InitHandshake = serde_json::from_str(&j).unwrap();
        assert_eq!(back, req);
    }
}
