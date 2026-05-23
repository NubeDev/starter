//! Phase 7d.2 (SCOPE-EXT §5) — per-tool [`PolicyEngine`] gate.
//!
//! Wraps any [`Tool`] in a check against the host's
//! [`starter_spi::authz::PolicyEngine`]. The check runs **before**
//! the tool body is invoked; a `Deny` short-circuits with
//! [`starter_spi::Error::Forbidden`] and the body never sees the
//! call. Mirrors the REST adapter's `with_permission` shape so a
//! manifest's `auth.permission` declaration produces the same
//! `(resource, action)` gate semantics regardless of surface (the
//! manifest field shipped here is the same one — SCOPE-EXT R15).
//!
//! Layer order (same convention as REST — SCOPE-EXT R15):
//!
//! ```text
//!   with_role   (outer, from `auth.require_role`)        ← NYI for MCP
//!     → with_scope (from `auth.require_scope`)            ← NYI for MCP
//!       → with_permission (inner, from `auth.permission`) ← THIS LAYER
//!         → tool body
//! ```
//!
//! `require_role` / `require_scope` for MCP tools are a follow-up;
//! the current MCP HTTP transport already authenticates the bearer
//! and routes only one role tier today. The `permission` field is
//! the load-bearing per-user gate; the others compose additively
//! once they exist.
//!
//! **Principal source.** `Tool::invoke` does not see the request, so
//! the principal is read from [`starter_mcp::current_principal`] —
//! a task-local the HTTP transport binds for the duration of every
//! dispatched call (see `starter_mcp::server::http::auth_layer`).
//! Calls arriving on the stdio transport have no principal; the
//! wrapper denies with `engine_missing_principal` so a tool that
//! requires an authz gate cannot be invoked from an unauthenticated
//! surface (fail-closed default).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use starter_authz::with_surface;
use starter_ext_spi::PermissionGate;
use starter_spi::authz::{Decision, PolicyEngine, ResourceRef};
use starter_spi::tool::{Tool, ToolDefinition};

/// Adapter-surface label written into
/// [`starter_authz::DecisionEntry::surface`] for every check this
/// wrapper performs. Kept as a `&'static str` constant so tests and
/// dashboards can refer to it without re-declaring the literal.
pub const SURFACE: &str = "mcp";

/// MCP `Tool` wrapper that gates dispatch on a
/// [`PolicyEngine::check`]. Constructed by `register_tools` when
/// the underlying `contributes.tools[]` entry declared
/// `auth.permission`. Bare tools (no `permission`) skip the wrapper
/// entirely — zero overhead for the no-gate case.
pub struct AuthzedToolBinding<T> {
    /// The unwrapped tool. Receives the call only after `Allow`.
    pub inner: T,
    /// Engine the gate consults on every invocation.
    pub engine: Arc<dyn PolicyEngine>,
    /// `(resource, action)` from the manifest.
    pub gate: PermissionGate,
}

impl<T> AuthzedToolBinding<T> {
    /// Wrap `inner` with a gate.
    pub fn new(inner: T, engine: Arc<dyn PolicyEngine>, gate: PermissionGate) -> Self {
        Self {
            inner,
            engine,
            gate,
        }
    }
}

#[async_trait]
impl<T> Tool for AuthzedToolBinding<T>
where
    T: Tool + Send + Sync,
{
    fn definition(&self) -> ToolDefinition {
        self.inner.definition()
    }

    async fn invoke(&self, input: Value) -> starter_spi::Result<Value> {
        let principal = starter_mcp::current_principal().ok_or(starter_spi::Error::Forbidden)?;
        let object = ResourceRef::collection(self.gate.resource.clone());
        // Run the check inside `with_surface("mcp", …)` so the audit
        // row lands with `surface = "mcp"` — that's what lets
        // `surface-decisions-share-audit-trail` distinguish a deny
        // here from a REST or gRPC deny against the same
        // (resource, action).
        let decision =
            with_surface(SURFACE, self.engine.check(&principal, &self.gate.action, &object)).await;
        match decision {
            Decision::Allow { .. } => self.inner.invoke(input).await,
            Decision::Deny { .. } => Err(starter_spi::Error::Forbidden),
        }
    }
}
