//! Sort direction. Kept deliberately tiny — there are only two
//! values and they never grow.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Ascending or descending sort.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Smallest first.
    #[default]
    Asc,
    /// Largest first.
    Desc,
}
