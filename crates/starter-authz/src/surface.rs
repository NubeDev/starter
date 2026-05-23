//! Adapter-surface task-local. Phase 7d.2 (SCOPE-EXT §5).
//!
//! Each adapter (REST, MCP, gRPC) wraps the engine.check() it
//! performs in [`with_surface`] so the engine's audit pipeline can
//! tag the resulting [`crate::DecisionEntry::surface`] with the
//! originating wire — `"rest"`, `"mcp"`, `"grpc"`. The engine reads
//! the value via [`current_surface`] inside its audit() helper.
//!
//! Task-local rather than passed-through-the-trait because the
//! `PolicyEngine::check` signature is shared across consumers and we
//! do not want to fork it just for an audit label.

use std::future::Future;

tokio::task_local! {
    static SURFACE: String;
}

/// Run `fut` with the surface label bound. Adapters call this
/// around any `engine.check(...)` they issue. Nested calls override
/// the outer label for their duration (REST → MCP wrappers are not
/// a real shape today; if they ever are, the inner surface wins).
pub async fn with_surface<F, T>(surface: impl Into<String>, fut: F) -> T
where
    F: Future<Output = T>,
{
    SURFACE.scope(surface.into(), fut).await
}

/// Return the currently bound surface, if any. Engine audit reads
/// this to populate [`crate::DecisionEntry::surface`].
pub fn current_surface() -> Option<String> {
    SURFACE.try_with(|s| s.clone()).ok()
}
