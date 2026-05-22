//! Typed registry of [`Reversible`] impls — one per resource kind.
//!
//! Transports (REST / gRPC / MCP / CLI) hold an [`Arc<ReversibleRegistry>`]
//! and never `match` on kinds themselves (SCOPE R3).

use std::collections::HashMap;
use std::sync::Arc;

use starter_spi::changelog::Reversible;

/// Lookup `kind -> &dyn Reversible`. Built once at server boot.
#[derive(Default, Clone)]
pub struct ReversibleRegistry {
    by_kind: HashMap<&'static str, Arc<dyn Reversible>>,
}

impl ReversibleRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one impl. Panics on duplicate registration — a real
    /// duplicate is a wire-up bug, not a runtime condition.
    pub fn insert(mut self, impl_: Arc<dyn Reversible>) -> Self {
        let kind = impl_.kind();
        if self.by_kind.contains_key(kind) {
            panic!("ReversibleRegistry: duplicate registration for kind {kind:?}");
        }
        self.by_kind.insert(kind, impl_);
        self
    }

    /// Look up by kind. `None` means the kind is not registered —
    /// callers MUST translate this into a stable error code, not a
    /// panic, since `kind` is consumer-supplied data.
    pub fn get(&self, kind: &str) -> Option<&Arc<dyn Reversible>> {
        self.by_kind.get(kind)
    }
}
