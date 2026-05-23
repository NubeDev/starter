//! Tool-dispatch task-local for the authenticated `Principal`.
//!
//! Phase 7d.2 (SCOPE-EXT §5 — MCP/gRPC parity): the HTTP transport
//! sets [`with_principal`] around its [`crate::server::dispatch`]
//! call so tool wrappers — concretely the `AuthzedToolBinding`
//! shipped by `starter-ext-mcp` — can run a `PolicyEngine::check`
//! before dispatching the tool body. The MCP `Tool` trait surface
//! is `invoke(input)` only; threading the principal through a
//! task-local keeps the trait unchanged.

use std::future::Future;

use starter_spi::auth::Principal;

tokio::task_local! {
    static PRINCIPAL: Principal;
}

/// Run `fut` with `principal` bound on the dispatch task. The MCP
/// HTTP transport wraps every `tools/call` dispatch with this; tool
/// wrappers read the binding via [`current_principal`].
pub async fn with_principal<F, T>(principal: Principal, fut: F) -> T
where
    F: Future<Output = T>,
{
    PRINCIPAL.scope(principal, fut).await
}

/// Return the principal bound on the current task, if any. Returns
/// `None` from non-HTTP transports (stdio is single-user and does
/// not set a principal) and from any task that wasn't entered via
/// [`with_principal`].
pub fn current_principal() -> Option<Principal> {
    PRINCIPAL.try_with(|p| p.clone()).ok()
}
