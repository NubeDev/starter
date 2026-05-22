//! ULID newtypes for the changelog.
//!
//! These ids are owned by the changelog module — they are intentionally
//! *not* the generic [`crate::Id<T>`] because the `starter_changes`
//! table is starter-owned and the id space is shared across resource
//! kinds. See `DOCS/backend/undo-redo/SCOPE.md` §"The seam".

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Identifier of a single row in `starter_changes`. ULID, monotonic
/// per recorder.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct ChangeId(pub String);

/// Identifier shared by every [`super::Change`] recorded inside a
/// single [`super::ChangeRecorder::transaction`] call. Undo operates
/// on whole groups.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct GroupId(pub String);

/// Correlation id linking a change to an external trace (HTTP request
/// id, agent run id, etc.). Opaque to the changelog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct TraceId(pub String);
