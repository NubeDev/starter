//! Wire types for the `secrets.get` host method.
//!
//! An extension reaches the host's secret store via
//! `ctx.secrets().get(name)`. For process-flavour callers the SDK
//! handle marshals each call as a JSON-RPC request on the
//! substrate's host-method channel; the supervisor's capability
//! gate enforces the `secrets` category, and the host's installed
//! [`HostMethodHandler`] resolves the request against the
//! `Capability::Secrets { prefixes }` manifest grant and the
//! backing [`SecretStore`].
//!
//! Builtin-flavour extensions never hit the wire.
//!
//! [`HostMethodHandler`]: ../../../starter-extensions/crates/starter-ext-supervisor/src/host_methods.rs
//! [`SecretStore`]: ../../../../crates/starter-spi/src/secrets/store.rs

use serde::{Deserialize, Serialize};

/// Wire payload an extension sends on `secrets.get`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsGetRequest {
    /// Secret name. Convention is dotted-namespace
    /// (`auth-token:pending`, `ai:anthropic:api_key`); the host
    /// gate enforces the prefix-allowlist manifest grant before
    /// resolving against the store.
    pub name: String,
}

/// Wire response for `secrets.get`.
///
/// The secret value is returned verbatim. Extensions are
/// responsible for not leaking it (the SDK side does *not*
/// re-wrap into `starter_spi::secrets::Secret` because doing so
/// would force a `starter-spi` dep on every extension — SCOPE
/// keeps the extension-author surface tiny).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsGetResponse {
    /// The secret value the store returned.
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_get_request_round_trip() {
        let req = SecretsGetRequest {
            name: "ai:anthropic:api_key".into(),
        };
        let j = serde_json::to_value(&req).unwrap();
        assert_eq!(j["name"], "ai:anthropic:api_key");
        let back: SecretsGetRequest = serde_json::from_value(j).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn secrets_get_response_round_trip() {
        let res = SecretsGetResponse {
            value: "<some-value>".into(),
        };
        let j = serde_json::to_value(&res).unwrap();
        let back: SecretsGetResponse = serde_json::from_value(j).unwrap();
        assert_eq!(back, res);
    }
}
