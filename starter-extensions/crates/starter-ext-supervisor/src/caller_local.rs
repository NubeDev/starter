//! Task-local `CallerIdentity` for adapter→supervisor handoff.
//!
//! The `Tool::invoke(&self, input)` trait surface used by the
//! adapters (`starter-ext-mcp::tool_wrapper::ProcessExtensionToolBinding`,
//! …) does not carry a caller argument. Hosts that authenticate
//! their REST callers (e.g. `rubix-agent`) bind the resolved
//! `CallerIdentity` on a task-local via [`scope`] before calling
//! `tool.invoke(...)`; the adapter reads it via [`current`] and
//! upgrades its supervisor call to `call_as` so the child's
//! `ctx.caller()` resolves to a real tenant frame.
//!
//! A missing scope ⇒ system frame ⇒ tenancy-scoped capability
//! handles refuse (warehouse_write, dashboard.read, …). That is
//! the correct fail-closed behaviour for unauthenticated callers.

use std::future::Future;

use starter_ext_spi::identity::CallerIdentity;

tokio::task_local! {
    static CURRENT: CallerIdentity;
}

/// Run `fut` with `caller` bound on the current task.
pub async fn scope<F, T>(caller: CallerIdentity, fut: F) -> T
where
    F: Future<Output = T>,
{
    CURRENT.scope(caller, fut).await
}

/// Return a clone of the caller bound on the current task, if any.
pub fn current() -> Option<CallerIdentity> {
    CURRENT.try_with(Clone::clone).ok()
}
