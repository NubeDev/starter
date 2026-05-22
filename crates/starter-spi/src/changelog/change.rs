//! The append-only change envelope. One row in `starter_changes`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::authz::ResourceRef;

use super::{Actor, ChangeId, GroupId, Op, TraceId};

/// A typed, append-only record of a single domain mutation.
///
/// Five product features collapse onto this primitive: user audit log,
/// AI-agent log, undo/redo, duplicate, and copy/paste. See
/// `DOCS/backend/undo-redo/SCOPE.md`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Change {
    /// ULID, monotonic per recorder.
    pub id: ChangeId,
    /// When the change was committed.
    pub at: DateTime<Utc>,
    /// Who caused the change.
    pub actor: Actor,
    /// Resource that was mutated. Kind + id (+ optional owner).
    pub resource: ResourceRef,
    /// Optimistic-concurrency token taken at read time.
    /// [`super::Reversible::apply_inverse`] uses it as the `WHERE`
    /// predicate and returns [`crate::Error::Conflict`] if the row
    /// has moved on. `None` for resources without versioning (caller
    /// accepts last-write-wins).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<u64>,
    /// What kind of mutation occurred.
    pub op: Op,
    /// Snapshot of the row *before* the mutation. Used for undo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub before: Option<serde_json::Value>,
    /// Snapshot of the row *after* the mutation. Used for redo,
    /// duplicate, and paste.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub after: Option<serde_json::Value>,
    /// Optional RFC 6902 patch document, as raw JSON. Kept as
    /// [`serde_json::Value`] so this crate does not depend on a
    /// patch-library crate — the backend picks one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub patch: Option<serde_json::Value>,
    /// Every change belongs to a group. Single-row mutations get a
    /// fresh `GroupId`; multi-row transactions share one. Undo
    /// operates on whole groups, so this is never `Option`.
    pub group_id: GroupId,
    /// Optional external trace id (HTTP request id, agent run id, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation: Option<TraceId>,
}
