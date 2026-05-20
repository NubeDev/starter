//! The [`PolicyEngine`] trait — the only seam a transport or a
//! handler talks to. Concrete engines (RBAC, DB-backed, Casbin)
//! live in `starter-authz`.

use async_trait::async_trait;

use crate::auth::Principal;

use super::{Decision, ResourceRef};

/// Decide whether a [`Principal`] may perform `action` on
/// `object`. Engines MUST be cheap to call — middleware invokes
/// `check` on every gated request.
///
/// `object.id == None` is the collection / route-level check.
/// `object.id == Some(_)` is the row-level check. `object.owner`,
/// when set by the caller, lets ownership rules
/// (`principal.subject == object.owner`) fire without a DB
/// round-trip from inside the engine.
#[async_trait]
pub trait PolicyEngine: Send + Sync + 'static {
    /// Evaluate the policy. Returning [`Decision::Deny`] with a
    /// stable `reason` code is the contract — the HTTP layer
    /// surfaces that code as `403 { "error": "<reason>" }`.
    async fn check(&self, principal: &Principal, action: &str, object: &ResourceRef) -> Decision;
}

/// Always-allow engine. The baseline used when `starter-authz` is
/// not wired in — preserves the existing pre-authz behaviour where
/// gating is done entirely by `require_role` / `require_scope`.
///
/// SCOPE.md R1: "a binary that disables `starter-authz` still
/// authenticates correctly; it just falls back to the existing
/// middleware."
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopPolicyEngine;

#[async_trait]
impl PolicyEngine for NoopPolicyEngine {
    async fn check(
        &self,
        _principal: &Principal,
        _action: &str,
        _object: &ResourceRef,
    ) -> Decision {
        Decision::allow()
    }
}
