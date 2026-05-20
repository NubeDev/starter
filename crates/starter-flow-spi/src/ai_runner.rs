//! `AiRunnerRegistry` — host-controlled lookup of AI providers.
//!
//! Mirrors the `ToolRegistry` shape used by the `tool-call` node
//! body: an `Arc<dyn AiRunnerRegistry>` is handed to the engine at
//! build time via `Engine::with_ai_runner_registry(...)` and then
//! read by the `ai-agent` node body to resolve a node's mandatory
//! `provider_id` config slot. Per D-F4.3 + D-F4.8.

use std::sync::Arc;

use starter_spi::ai::AiRunner;

use crate::node::KindId;

/// Host-controlled registry the `ai-agent` body resolves
/// `provider_id` against.
///
/// SCOPE R5 keeps the body stateless: the registry is an
/// `Arc<dyn>` handed in at construction time and the trait surface
/// is read-only. The host constructs and freezes the registry at
/// engine-build time; the body never mutates it. The registry's
/// value type is `Arc<dyn AiRunner>` so the same runner can be
/// shared across many concurrent invocations on a single
/// registration. Per D-F4.3.
pub trait AiRunnerRegistry: Send + Sync + 'static {
    /// Look up an `AiRunner` by its reverse-DNS `provider_id`.
    /// Returns `None` if no runner is registered under the given id.
    fn lookup(&self, provider_id: &KindId) -> Option<Arc<dyn AiRunner>>;
}
