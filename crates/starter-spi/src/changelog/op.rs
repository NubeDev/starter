//! Mutation kind recorded for a [`super::Change`].

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The operation a [`super::Change`] represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// New row created.
    Create,
    /// Existing row updated.
    Update,
    /// Row deleted.
    Delete,
    /// Domain-specific operation. `String` (not `&'static str`)
    /// because values are read back from `jsonb`.
    Custom(String),
}
