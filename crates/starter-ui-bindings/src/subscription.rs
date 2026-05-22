//! Subscription-plan derivation.
//!
//! Every slot read during a resolve is appended to the
//! [`EvalContext::access_log`](crate::EvalContext::access_log). After
//! the tree has been substituted the host calls
//! [`SubscriptionPlan::from_log`] to dedupe the entries and emit one
//! [`Subject`] per unique `(entity_id, slot)` pair. The client
//! subscribes to those subjects; live updates from the host's pub/sub
//! plane invalidate the bound widgets without re-resolving the whole
//! tree.
//!
//! The dedupe is **per-resolve** by construction — every `$target`
//! binding seeds the cursor with the per-resolve target id, so the
//! subjects emitted for resolve-of-target-A are disjoint from the
//! subjects emitted for resolve-of-target-B unless a binding crosses
//! into a shared subtree. This is the property the "one page, N
//! targets" test exercises.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::graph::EntityId;

/// One `(entity, slot)` pair the resolver read during evaluation.
/// Captured into [`EvalContext::access_log`](crate::EvalContext::access_log)
/// at every `.slot` step against the graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotAccess {
    pub entity_id: EntityId,
    pub slot: String,
}

/// One subject the client should subscribe to. The wire shape is
/// `"<entity_id>/<slot>"` — a stable NATS-like topic that the host's
/// pub/sub plane already publishes per slot write. Hosts that use a
/// different subject convention (dot-separated, hierarchical) can
/// override [`Subject::wire`] when emitting.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Subject {
    pub entity_id: EntityId,
    pub slot: String,
}

impl Subject {
    /// Default NATS-ish wire string `"<entity_id>/<slot>"`. Hosts
    /// with a different subject grammar can format their own.
    pub fn wire(&self) -> String {
        format!("{}/{}", self.entity_id, self.slot)
    }
}

/// The deduped list of subjects emitted alongside a resolved tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub subjects: Vec<Subject>,
}

impl SubscriptionPlan {
    /// Build a plan from a raw access log. Deduplicates `(entity,
    /// slot)` pairs and emits them in deterministic (sorted) order so
    /// two resolves of the same page against the same target produce
    /// byte-identical plans — useful for cache keys and snapshot
    /// tests.
    pub fn from_log<I>(log: I) -> Self
    where
        I: IntoIterator<Item = SlotAccess>,
    {
        let set: BTreeSet<SlotAccess> = log.into_iter().collect();
        let subjects = set
            .into_iter()
            .map(|a| Subject {
                entity_id: a.entity_id,
                slot: a.slot,
            })
            .collect();
        Self { subjects }
    }

    /// `true` when nothing was bound — the page is fully static.
    pub fn is_empty(&self) -> bool {
        self.subjects.is_empty()
    }

    /// Number of distinct subjects.
    pub fn len(&self) -> usize {
        self.subjects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupes_and_sorts() {
        let plan = SubscriptionPlan::from_log(vec![
            SlotAccess {
                entity_id: "b".into(),
                slot: "y".into(),
            },
            SlotAccess {
                entity_id: "a".into(),
                slot: "x".into(),
            },
            SlotAccess {
                entity_id: "a".into(),
                slot: "x".into(),
            },
        ]);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan.subjects[0].wire(), "a/x");
        assert_eq!(plan.subjects[1].wire(), "b/y");
    }
}
