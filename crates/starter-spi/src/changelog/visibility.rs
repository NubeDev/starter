//! Read-side ACL for the audit and agent-log projections.
//!
//! The changelog stores arbitrary `before` / `after` snapshots of
//! consumer data, which makes the projections a confused-deputy risk
//! and a GDPR target. Consumers register a [`ChangelogVisibility`]
//! per resource kind; the projection crates MUST call it before
//! returning a row. Default policy is "deny unknown kinds" so a
//! missing registration fails closed.

use crate::auth::Principal;

use super::Change;

/// Per-resource-kind ACL gate for changelog reads.
pub trait ChangelogVisibility: Send + Sync {
    /// Decide whether `principal` is allowed to read `ch`.
    fn may_read(&self, principal: &Principal, ch: &Change) -> bool;
}
