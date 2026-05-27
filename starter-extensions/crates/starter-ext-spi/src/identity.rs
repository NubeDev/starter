//! Caller identity carried on every inbound JSON-RPC frame from host to
//! extension.
//!
//! Per [`docs/design/extensions/caller-identity.md`](../../../../rubix/docs/design/extensions/caller-identity.md)
//! and the extensions north-star scope (row 1 of the critical path):
//! the host stamps a [`CallerIdentity`] onto every request or
//! notification it sends to an extension so the extension's handlers
//! can scope work to the requesting tenant / user. This is the lynchpin
//! of "Rule 3" in the north-star proposal — nothing tenant-scoped
//! ships before identity propagation.
//!
//! ## Wire shape
//!
//! Identity rides in an optional `_meta.caller` field on
//! [`crate::jsonrpc::JsonRpcRequest`] and
//! [`crate::jsonrpc::JsonRpcNotification`]:
//!
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "id": 1,
//!   "method": "tools/com.acme.chart.render",
//!   "params": { "rows": [...] },
//!   "_meta": {
//!     "caller": {
//!       "tenant_id": "t-42",
//!       "user_id": "u-7",
//!       "roles": ["viewer"],
//!       "request_id": "req-9c1f"
//!     }
//!   }
//! }
//! ```
//!
//! The field is optional: a frame without `_meta.caller` represents a
//! host-internal / system call (health pings, lifecycle, init). The SDK
//! reflects this through `ctx.caller()` returning `Option<&CallerIdentity>`.
//!
//! ## Why a sidecar field, not nested in `params`
//!
//! Nesting under `params._meta` (MCP's convention) couples identity to
//! the handler's parameter schema — every contributed tool would have
//! to make room for the reserved key. Lifting identity onto the
//! envelope keeps `params` exclusively the handler's contract, which
//! matches how the request id is treated.

use serde::{Deserialize, Serialize};

/// Identifies the principal on whose behalf a JSON-RPC frame is being
/// dispatched. Stamped by the host onto every outbound frame so the
/// extension's `ctx.caller()` can resolve it.
///
/// All identity fields except [`request_id`](Self::request_id) are
/// optional — a host-internal frame (health ping, init) carries only a
/// request id, and even that is allowed to be empty when the host
/// itself has no correlation id to share.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallerIdentity {
    /// Owning tenant of the request. `None` for host-internal frames
    /// and for anonymous / unauthenticated paths the host explicitly
    /// permits. Tenant-scoped capability handles (e.g.
    /// `WarehouseReadHandle`) refuse to serve a frame whose
    /// `tenant_id` is `None` — that refusal is what closes the soft
    /// trust boundary the loopback shortcut exposes today.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// Acting user inside `tenant_id`. `None` when the request is
    /// system-driven (cron, lifecycle hook) but still tenant-scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Roles granted to `user_id` for this request. Empty when no
    /// roles are attached (system frames, or users whose role list
    /// the host did not resolve). Authorisation handles decide what
    /// "no roles" means; the kernel ships the list verbatim.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,

    /// End-to-end correlation id. Surfaces in extension tracing /
    /// logs so a failed request can be tracked from the inbound
    /// REST/MCP/CLI dispatcher through to the child process and
    /// back. Empty string when the host has no correlation id to
    /// share — extensions treat that as "no correlation available"
    /// rather than as a distinct identity.
    #[serde(default)]
    pub request_id: String,
}

impl CallerIdentity {
    /// Construct a system / host-internal identity (no tenant, no
    /// user, no roles). Use when the host emits a frame on its own
    /// behalf — health, init, shutdown, periodic cron — rather than
    /// on behalf of a request from an external caller.
    pub fn system() -> Self {
        Self::default()
    }

    /// `true` if no tenant or user is attached. Tenant-scoped
    /// capability handles use this as the gate that refuses the frame.
    pub fn is_system(&self) -> bool {
        self.tenant_id.is_none() && self.user_id.is_none()
    }
}

/// Sidecar envelope carried in the `_meta` field of a request or
/// notification.
///
/// Kept as a struct rather than a free-form `serde_json::Value` so
/// adapters cannot accidentally invent sibling keys outside the
/// kernel's namespace. Future kernel-level metadata (tracing
/// baggage, deadline propagation) lands as additional optional
/// fields here, not as new top-level envelope members.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameMeta {
    /// Identity of the principal this frame is dispatched on behalf
    /// of. Absent for purely host-internal frames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller: Option<CallerIdentity>,
}

impl FrameMeta {
    /// Construct a `FrameMeta` carrying just a caller identity.
    pub fn with_caller(caller: CallerIdentity) -> Self {
        Self {
            caller: Some(caller),
        }
    }

    /// `true` when no fields are populated; used by the
    /// `skip_serializing_if` predicate on envelope `_meta` fields so
    /// an empty meta sidecar never appears on the wire.
    pub fn is_empty(&self) -> bool {
        self.caller.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_identity_is_empty() {
        let id = CallerIdentity::system();
        assert!(id.is_system());
        assert!(id.tenant_id.is_none());
        assert!(id.user_id.is_none());
        assert!(id.roles.is_empty());
        assert!(id.request_id.is_empty());
    }

    #[test]
    fn tenant_only_round_trip() {
        let id = CallerIdentity {
            tenant_id: Some("t-42".into()),
            user_id: Some("u-7".into()),
            roles: vec!["viewer".into()],
            request_id: "req-9c1f".into(),
        };
        let j = serde_json::to_string(&id).unwrap();
        // Empty fields are omitted; populated ones are present.
        assert!(j.contains("\"tenant_id\":\"t-42\""));
        assert!(j.contains("\"roles\":[\"viewer\"]"));
        let back: CallerIdentity = serde_json::from_str(&j).unwrap();
        assert_eq!(back, id);
        assert!(!back.is_system());
    }

    #[test]
    fn empty_fields_are_omitted_on_the_wire() {
        let id = CallerIdentity {
            tenant_id: Some("t-1".into()),
            ..Default::default()
        };
        let j = serde_json::to_string(&id).unwrap();
        assert!(!j.contains("user_id"));
        assert!(!j.contains("roles"));
        // request_id has no skip predicate (empty string is meaningful
        // "no correlation") — the field is always present.
        assert!(j.contains("\"request_id\":\"\""));
    }

    #[test]
    fn frame_meta_empty_predicate() {
        assert!(FrameMeta::default().is_empty());
        let m = FrameMeta::with_caller(CallerIdentity {
            tenant_id: Some("t-1".into()),
            ..Default::default()
        });
        assert!(!m.is_empty());
        assert_eq!(m.caller.as_ref().unwrap().tenant_id.as_deref(), Some("t-1"));
    }
}
