//! Built-in node descriptor registry.
//!
//! The `tool_registry` chokepoint sits between the engine and host-
//! provided tools; this module is its analogue for *node kinds*.
//! Adapters that need to enumerate available nodes (catalog browsers,
//! flow designers, an eventual `starter-mcp` `flow.nodes.list` tool,
//! REST surfaces) reach for [`builtin_descriptors`] or build a
//! [`StaticNodeKindRegistry`] from it.
//!
//! Each entry is a `&'static NodeDescriptor` exported by the matching
//! module (e.g. [`crate::log::DESCRIPTOR`]). Feature gates compose:
//! the slice contains only the kinds the consuming binary opted into,
//! matching the existing `cfg(feature = "...")` posture on the module
//! declarations in [`crate`].
//!
//! ## Help text / i18n
//!
//! Descriptors carry *catalog keys* (e.g. `"starter.flow.node.log.label"`),
//! not translated strings. Resolution happens at the presentation
//! layer through `starter-i18n`'s `TranslateBundle`. The seed
//! catalogs at `crates/starter-i18n/catalogs/starter/{en,es}.json`
//! ship label / summary / help text for every built-in kind.
//!
//! ## MCP access (planned)
//!
//! The MCP crate (`starter-mcp`) can expose this registry as a tool
//! by depending on `starter-flow-nodes`, calling
//! [`builtin_descriptors`] for the list, and resolving each
//! descriptor's `*_key` fields through the request's locale. Designed
//! to plug in; not yet wired.

use std::collections::HashMap;

use starter_flow_spi::node::{KindId, NodeDescriptor, NodeKindRegistry};

/// Every built-in [`NodeDescriptor`] enabled in the current build.
///
/// Order is the declaration order in [`crate`]'s `lib.rs`. Callers
/// that need a stable sort should sort by `descriptor.kind`.
#[allow(clippy::vec_init_then_push)]
pub fn builtin_descriptors() -> Vec<&'static NodeDescriptor> {
    let mut out: Vec<&'static NodeDescriptor> = Vec::new();

    #[cfg(feature = "transform")]
    out.push(&crate::transform::DESCRIPTOR);
    #[cfg(feature = "tool-call")]
    out.push(&crate::tool_call::DESCRIPTOR);
    #[cfg(feature = "ai-agent")]
    out.push(&crate::ai_agent::DESCRIPTOR);
    #[cfg(feature = "branch")]
    out.push(&crate::branch::DESCRIPTOR);
    #[cfg(feature = "merge")]
    out.push(&crate::merge::DESCRIPTOR);
    #[cfg(feature = "gate")]
    out.push(&crate::gate::DESCRIPTOR);
    #[cfg(feature = "subflow")]
    out.push(&crate::subflow::DESCRIPTOR);
    #[cfg(feature = "trigger-explicit")]
    out.push(&crate::trigger_explicit::DESCRIPTOR);
    #[cfg(feature = "trigger-event")]
    out.push(&crate::trigger_event::DESCRIPTOR);
    #[cfg(feature = "trigger-schedule")]
    out.push(&crate::trigger_schedule::DESCRIPTOR);
    #[cfg(feature = "trigger-webhook")]
    out.push(&crate::trigger_webhook::DESCRIPTOR);
    #[cfg(feature = "http-out")]
    out.push(&crate::http_out::DESCRIPTOR);
    #[cfg(feature = "log")]
    out.push(&crate::log::DESCRIPTOR);
    #[cfg(feature = "sleep")]
    out.push(&crate::sleep::DESCRIPTOR);

    out
}

/// In-memory [`NodeKindRegistry`] populated from a slice of static
/// descriptors. Mirrors [`crate::tool_registry::StaticToolRegistry`]'s
/// posture: built once at engine-build time, then handed out as
/// `Arc<dyn NodeKindRegistry>`.
pub struct StaticNodeKindRegistry {
    by_kind: HashMap<String, &'static NodeDescriptor>,
    order: Vec<&'static NodeDescriptor>,
}

impl StaticNodeKindRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self {
            by_kind: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Construct a registry containing every built-in descriptor
    /// enabled in the current build (see [`builtin_descriptors`]).
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        for d in builtin_descriptors() {
            reg.register(d);
        }
        reg
    }

    /// Register a descriptor. Replaces any previous entry under the
    /// same kind id (the registry is host-owned, so collisions are a
    /// host-build bug, not a runtime error).
    pub fn register(&mut self, descriptor: &'static NodeDescriptor) {
        if self
            .by_kind
            .insert(descriptor.kind.to_owned(), descriptor)
            .is_none()
        {
            self.order.push(descriptor);
        }
    }
}

impl Default for StaticNodeKindRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeKindRegistry for StaticNodeKindRegistry {
    fn lookup(&self, kind: &KindId) -> Option<&NodeDescriptor> {
        self.by_kind.get(kind.as_str()).copied()
    }

    fn all(&self) -> Vec<&NodeDescriptor> {
        self.order.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_are_valid_kind_ids() {
        for d in builtin_descriptors() {
            KindId::new(d.kind)
                .unwrap_or_else(|e| panic!("descriptor.kind {:?} not a valid KindId: {e}", d.kind));
        }
    }

    #[test]
    fn descriptor_keys_follow_namespace_convention() {
        for d in builtin_descriptors() {
            let prefix = "starter.flow.node.";
            assert!(
                d.label_key.starts_with(prefix),
                "label_key {:?} should live under {prefix}*",
                d.label_key
            );
            assert!(d.summary_key.starts_with(prefix));
            assert!(d.help_key.starts_with(prefix));
        }
    }

    #[test]
    fn registry_lookup_round_trips() {
        let reg = StaticNodeKindRegistry::with_builtins();
        for d in builtin_descriptors() {
            let kind = KindId::new(d.kind).unwrap();
            let found = reg.lookup(&kind).expect("descriptor registered");
            assert_eq!(found.kind, d.kind);
        }
    }

    #[test]
    fn all_kinds_unique() {
        let mut seen = std::collections::HashSet::new();
        for d in builtin_descriptors() {
            assert!(seen.insert(d.kind), "duplicate kind id: {}", d.kind);
        }
    }
}
