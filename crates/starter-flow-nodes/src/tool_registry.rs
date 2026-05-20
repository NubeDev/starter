//! Host-controlled tool registry shared by the `tool-call` and
//! `ai-agent` node bodies.
//!
//! Lives in its own always-compiled module (rather than inside
//! `tool_call.rs`) so the `ai-agent` feature can depend on the
//! [`ToolRegistry`] trait without dragging in the full `tool-call`
//! node body. SCOPE R5 still applies: the registry is constructed
//! and frozen at engine-build time, then handed to every node body
//! that needs it as an `Arc<dyn ToolRegistry>`.

use std::collections::HashMap;
use std::sync::Arc;

use starter_flow_spi::node::KindId;
use starter_spi::tool::Tool;

/// Host-controlled tool registry the `tool-call` body resolves
/// `tool_id` against, and the `ai-agent` body uses to dispatch
/// model-emitted tool calls (D-F4.9 — the same chokepoint).
///
/// SCOPE R5 keeps both bodies stateless: the registry is an
/// `Arc<dyn>` handed in at construction time and the trait surface
/// is read-only. The host constructs and freezes the registry at
/// engine-build time; bodies never mutate it. The registry's value
/// type is `Arc<dyn Tool>` so the same `Tool` can be shared across
/// many concurrent invocations on a single registration.
pub trait ToolRegistry: Send + Sync + 'static {
    /// Look up a tool by its reverse-DNS [`KindId`]. Returns `None`
    /// if no tool is registered under the given id.
    fn lookup(&self, tool_id: &KindId) -> Option<Arc<dyn Tool>>;
}

/// In-memory [`ToolRegistry`] populated at engine-build time.
///
/// Mutation is confined to the builder phase ([`Self::register`]);
/// once the registry is wrapped in an `Arc<dyn ToolRegistry>` and
/// handed to a node body, it is read-only.
#[derive(Default)]
pub struct StaticToolRegistry {
    tools: HashMap<KindId, Arc<dyn Tool>>,
}

impl StaticToolRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool under its [`KindId`]. Replaces any previous
    /// entry under the same id (the registry is host-owned, so
    /// collisions are a host-build bug, not a runtime error).
    pub fn register(&mut self, tool_id: KindId, tool: Arc<dyn Tool>) {
        self.tools.insert(tool_id, tool);
    }
}

impl ToolRegistry for StaticToolRegistry {
    fn lookup(&self, tool_id: &KindId) -> Option<Arc<dyn Tool>> {
        self.tools.get(tool_id).cloned()
    }
}
