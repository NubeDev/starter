//! Typed capability grants.
//!
//! Per SCOPE.md **R6**, the manifest distinguishes two things:
//!
//! - `requires:` — capability *categories* the extension needs to function.
//! - `capabilities:` — runtime *grants* the operator supplies (allowlists,
//!   empty lists for neutralised grants, or scalars for boolean grants).
//!
//! This module models the *grant* side as a typed enum. The host normalises
//! the manifest's `capabilities:` map into a `Vec<Capability>` before any
//! adapter or SDK sees it. Untyped strings never reach the extension's
//! `Ctx`.

use serde::{Deserialize, Serialize};

/// One typed capability grant.
///
/// Variants are stable; adding a category is additive within the manifest
/// schema major. Operators see the category name (e.g. `http_out`) in YAML;
/// the host receives the parsed variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Capability {
    /// Secret store name prefixes the extension is allowed to read.
    /// `Secrets(vec![])` is a legal neutralised grant.
    Secrets {
        /// Allowed `SecretStore` name prefixes (e.g. `"weather:*"`).
        prefixes: Vec<String>,
    },

    /// Outbound HTTP authorities the extension is allowed to call. An empty
    /// vec is a legal neutralised grant: the extension loads, every call is
    /// denied at runtime.
    HttpOut {
        /// Allowed authorities (`host[:port]`).
        authorities: Vec<Authority>,
    },

    /// Filesystem path specs the extension is allowed to read/write. Empty
    /// vec is the neutralised form.
    Fs {
        /// Allowed path specs.
        paths: Vec<PathSpec>,
    },

    /// Permission to read wall-clock time. Boolean grant.
    WallClock {
        /// `true` to grant; the field exists so the YAML reads
        /// `wall_clock: true` rather than `wall_clock: {}`.
        granted: bool,
    },

    /// An opaque, host-defined capability. Used as the escape hatch for
    /// consumer-specific grants the kernel does not know about.
    Custom {
        /// The capability name. Adapters interpret it.
        name: String,
        /// Optional opaque payload (allowlist, policy id, …).
        params: serde_json::Value,
    },
}

/// An HTTP authority allowlist entry (`host[:port]`).
///
/// Newtype rather than `String` so adapters can attach validation/parsing
/// without changing the wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Authority(pub String);

impl Authority {
    /// Borrow the raw `host[:port]` string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A filesystem path spec entry.
///
/// Kept as an opaque string in `starter-ext-spi`; `starter-ext-host` is the
/// crate that resolves it (relative-to-bundle vs. absolute, read-only vs.
/// read-write, glob support). Adapters that enforce capability use see only
/// the string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PathSpec(pub String);

impl PathSpec {
    /// Borrow the raw path-spec string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_round_trip() {
        let cap = Capability::Secrets {
            prefixes: vec!["weather:*".to_string()],
        };
        let j = serde_json::to_value(&cap).unwrap();
        assert_eq!(j["kind"], "secrets");
        let back: Capability = serde_json::from_value(j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn http_out_empty_allowlist_is_legal() {
        let cap = Capability::HttpOut {
            authorities: vec![],
        };
        // SCOPE R6: empty list is the "neutralised grant" form. Just confirm
        // it serialises round-trip; runtime denial happens at the adapter.
        let j = serde_json::to_string(&cap).unwrap();
        let back: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn custom_carries_opaque_params() {
        let cap = Capability::Custom {
            name: "kv".to_string(),
            params: serde_json::json!({ "bucket": "weather" }),
        };
        let j = serde_json::to_string(&cap).unwrap();
        let back: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(back, cap);
    }
}
