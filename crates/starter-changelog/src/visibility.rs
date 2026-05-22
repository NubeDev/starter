//! Default-deny [`ChangelogVisibility`] registry.
//!
//! Per `DOCS/backend/undo-redo/SCOPE.md` §"Security & privacy", a
//! missing per-kind registration MUST fail closed. The projection
//! crates (`starter-audit`, `starter-agent-log`) MUST route every
//! candidate row through [`ChangelogVisibilityRegistry::may_read`]
//! before returning it.

use std::collections::HashMap;
use std::sync::Arc;

use starter_spi::auth::Principal;
use starter_spi::changelog::{Change, ChangelogVisibility};

/// Per-resource-kind ACL gate.
///
/// Kinds with no registered impl are denied. Build the registry once
/// at server start and share it as `Arc<ChangelogVisibilityRegistry>`.
#[derive(Default, Clone)]
pub struct ChangelogVisibilityRegistry {
    rules: HashMap<String, Arc<dyn ChangelogVisibility>>,
}

impl ChangelogVisibilityRegistry {
    /// Empty registry. All reads are denied until kinds are registered.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a visibility rule for `kind`. Replaces any prior rule
    /// for the same kind.
    pub fn insert(
        mut self,
        kind: impl Into<String>,
        rule: Arc<dyn ChangelogVisibility>,
    ) -> Self {
        self.rules.insert(kind.into(), rule);
        self
    }

    /// Decide whether `principal` may read `ch`. Returns `false` when
    /// no rule is registered for `ch.resource.kind` — the fail-closed
    /// default the SCOPE mandates.
    pub fn may_read(&self, principal: &Principal, ch: &Change) -> bool {
        match self.rules.get(&ch.resource.kind) {
            Some(rule) => rule.may_read(principal, ch),
            None => {
                tracing::warn!(
                    kind = %ch.resource.kind,
                    "changelog read denied: no ChangelogVisibility registered for kind"
                );
                false
            }
        }
    }
}
