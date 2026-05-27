//! Wire types for the `authz.check` host method.
//!
//! Per [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
//! row 5: an extension asks the host's authz engine "is the
//! calling principal allowed to do X on Y?" via
//! `ctx.authz().check(action, resource)`. Process / WASM flavours
//! marshal that as a JSON-RPC call on the substrate's host-method
//! channel; the supervisor routes it into the host's
//! [`super::Capability::AuthzCheck`] backend after the capability
//! gate fires.
//!
//! Builtin extensions bypass the wire and call the host's
//! `AuthzBackend` directly.

use serde::{Deserialize, Serialize};

/// Wire payload an extension sends on `authz.check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthzCheckRequest {
    /// Verb the extension is asking about (e.g. `"view"`,
    /// `"edit"`). Engines pattern-match on the action string.
    pub action: String,
    /// Resource the action targets, in `kind` or `kind:id`
    /// form. The host parses this into a `ResourceRef`; tenancy
    /// is populated from the caller's `_meta.caller.tenant_id`,
    /// the extension cannot override it.
    pub resource: String,
}

/// Wire response for `authz.check`.
///
/// Boolean rather than a typed `Decision` so the wire shape stays
/// engine-neutral — host engines that emit richer decision
/// metadata reduce it to allow/deny at the wire boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthzCheckResponse {
    /// `true` ⇒ the action is permitted for the calling
    /// principal; `false` ⇒ deny. Extensions should treat
    /// any error response as deny.
    pub allowed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authz_check_request_round_trip() {
        let req = AuthzCheckRequest {
            action: "view".into(),
            resource: "rubix.dashboard.page:disk-overview".into(),
        };
        let j = serde_json::to_value(&req).unwrap();
        assert_eq!(j["action"], "view");
        let back: AuthzCheckRequest = serde_json::from_value(j).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn authz_check_response_round_trip() {
        for allowed in [true, false] {
            let res = AuthzCheckResponse { allowed };
            let j = serde_json::to_value(res).unwrap();
            let back: AuthzCheckResponse = serde_json::from_value(j).unwrap();
            assert_eq!(back, res);
        }
    }
}
