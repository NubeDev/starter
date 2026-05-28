//! Per-task `CURRENT_ACTOR` for the duration of a tool dispatch.
//!
//! Transports (REST handler in `rubix-agent::routes::tools`, the
//! MCP server, the flow runner's tool-call hop) install the
//! caller's [`Actor`] on the running task before invoking
//! [`starter_spi::tool::Tool::invoke`]. The
//! [`crate::dispatch::record_if_reversible`] write recorded by an
//! [`UndoDispatcher`] reads this task-local to stamp the row's
//! `actor` field; the per-actor cursor (see [`crate::UndoService`])
//! is keyed off the same value.
//!
//! [`UndoDispatcher`]: ../../../rubix_tools/undo/dispatch/struct.UndoDispatcher.html
//!
//! Long-form: a `Tool::invoke(input)` signature carries no caller
//! context. Threading an `Actor` parameter through every
//! `Tool` impl is the wrong shape — most tools never need it.
//! Task-local injection puts the cost on the transports that
//! actually have the identity, and zero-cost-when-absent on every
//! other dispatch site.

use starter_spi::changelog::Actor;
use std::future::Future;

tokio::task_local! {
    /// Current actor for the active task. Set by [`scope`]; read
    /// by [`try_current`].
    static CURRENT_ACTOR: Actor;
}

/// Run `f` with `actor` installed as the current task's actor.
///
/// Equivalent to `CURRENT_ACTOR.scope(actor, f).await`. Provided
/// as a free function so callers do not import the task-local
/// directly; the static itself is a private implementation detail.
pub async fn scope<F>(actor: Actor, f: F) -> F::Output
where
    F: Future,
{
    CURRENT_ACTOR.scope(actor, f).await
}

/// Return the current task's actor, if any.
///
/// `None` outside a [`scope`] — callers SHOULD fall back to
/// `Actor::System` so a misconfigured wiring still records an
/// audit-safe value rather than dropping the row.
pub fn try_current() -> Option<Actor> {
    CURRENT_ACTOR.try_with(Clone::clone).ok()
}
