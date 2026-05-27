//! Task-local `CallerIdentity` inside the SDK child process.
//!
//! The process dispatcher binds the per-call caller (extracted from
//! `_meta.caller` on the inbound tools/ frame) on a task-local
//! before invoking the handler. The [`HostRpc::call_sync`] path
//! reads it via [`current`] and re-stamps `_meta.caller` on every
//! outbound host frame (`warehouse.write`, `dashboard.read`, …) so
//! the supervisor's host-method handler sees the same tenant the
//! tools/ call was made under.
//!
//! Absent ⇒ no `_meta` on the outbound frame ⇒ supervisor treats
//! the call as a system frame, which the host-side tenant-scoped
//! backends fail-closed on.
//!
//! [`HostRpc::call_sync`]: crate::host_rpc::HostRpc::call_sync
//! [`current`]: crate::caller_local::current

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

/// Synchronous variant for `spawn_blocking` handler bodies — runs
/// the closure with `caller` bound on the current (blocking) task.
pub fn scope_sync<F, T>(caller: CallerIdentity, f: F) -> T
where
    F: FnOnce() -> T,
{
    CURRENT.sync_scope(caller, f)
}

/// Clone of the caller bound on the current task, if any.
pub fn current() -> Option<CallerIdentity> {
    CURRENT.try_with(Clone::clone).ok()
}
