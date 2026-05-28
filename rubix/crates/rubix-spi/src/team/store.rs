//! Trait + value types for the team-admin verb surface.
//!
//! Lives in `rubix-spi` so that `rubix-tools` (in-memory fake +
//! `TeamReversible`) and `rubix-store-postgres` (Pg-backed impl)
//! both share the contract without depending on each other
//! \u{2014} they are siblings, both rooted in `rubix-spi`.

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_spi::error::Result;

/// Resource-kind discriminator for team rows / memberships.
/// Matches `ResourceRef::kind` on every recorded `Change` for a
/// team row.
pub const TEAM_KIND: &str = "team";

/// One team row + its current membership map.
///
/// The membership map is part of the team's snapshot so a single
/// `Change` envelope can undo either a `create` (`Op::Create`,
/// snapshot in `after`) or an `assign` (`Op::Update`, snapshots
/// in `before` / `after`). `BTreeMap` is required so the
/// snapshot JSON is deterministic (matches the choice in
/// `Diagnostic::params`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamRow {
    /// Stable id.
    pub team_id: String,
    /// Human-facing name. UNIQUE in the Pg schema; the in-memory
    /// fake enforces the same via a `list().any(...)` scan.
    pub name: String,
    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `user_id -> assigned_at_ms`. Deterministic ordering is
    /// load-bearing for the `(prior, new)` byte-exact comparison
    /// on the no-op assign / unassign paths.
    #[serde(default)]
    pub members: BTreeMap<String, i64>,
}

/// Sparse update payload \u{2014} only the fields the verb
/// actually touched are populated; the rest stay as the current
/// row had them. See
/// [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
/// \u{00A7}"Snapshot shape".
///
/// Used by `TeamReversible::apply_inverse` so two concurrent
/// edits to unrelated fields do not clobber each other on undo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamPatch {
    /// Replace the `members` map verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<BTreeMap<String, i64>>,
    /// Replace the team name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Replace the team description. Nested `Option` so the
    /// patch can distinguish "leave description alone" (`None`)
    /// from "explicitly clear description" (`Some(None)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
}

/// Persistence surface the team verbs target.
#[async_trait]
pub trait TeamAdminStore: Send + Sync {
    /// Insert a new team. Returns the row that landed.
    /// `Error::Conflict` on duplicate `name`.
    async fn create(&self, row: TeamRow) -> Result<TeamRow>;
    /// Add `user_id` to `team_id`. Returns `(prior_row,
    /// new_row)`; on a no-op re-assignment both halves are
    /// equal and the verb skips the audit row. `Error::NotFound`
    /// when the team itself does not resolve.
    async fn assign(
        &self,
        team_id: &str,
        user_id: &str,
        now_ms: i64,
    ) -> Result<(TeamRow, TeamRow)>;
    /// Remove `user_id` from `team_id`. Returns `(prior_row,
    /// new_row)`; on a no-op (user was not a member) both
    /// halves are equal. Returns `Error::NotFound` when the
    /// *team* does not resolve \u{2014} the absence of a member
    /// is a no-op, but the absence of the team itself is a
    /// wire-shaped bug.
    async fn unassign(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<(TeamRow, TeamRow)>;
    /// Fetch by team_id.
    async fn get(&self, team_id: &str) -> Result<Option<TeamRow>>;
    /// List all team rows. Order is unspecified \u{2014} callers
    /// sort if they need stability. Used by the update verb for
    /// the rename uniqueness check and by the delete verb for
    /// cascade analysis.
    async fn list(&self) -> Result<Vec<TeamRow>>;
    /// Restore (or insert) a row to the supplied snapshot. Used
    /// by `TeamReversible::apply_inverse` to walk a `Change`
    /// backwards.
    async fn put(&self, row: TeamRow) -> Result<()>;
    /// Hard-delete a row by id. Returns `Error::NotFound` when
    /// the id does not resolve \u{2014} the verb relies on this
    /// signal to distinguish a missing-target call from a
    /// successful no-op (the same posture the verb takes on
    /// `member.unassign` against a missing team).
    async fn delete(&self, team_id: &str) -> Result<()>;
}
