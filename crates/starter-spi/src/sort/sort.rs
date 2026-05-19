//! `Sort` — the field/direction pair list endpoints accept.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::direction::Direction;

/// One sort clause. List endpoints accept a single `Sort`; consumers
/// that need multi-column sorting wrap a `Vec<Sort>` themselves.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Sort {
    /// Field name, matched against the endpoint's schema.
    pub field: String,

    /// Direction; defaults to ascending.
    #[serde(default)]
    pub direction: Direction,
}

impl Sort {
    /// Ascending sort on `field`.
    pub fn asc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: Direction::Asc,
        }
    }

    /// Descending sort on `field`.
    pub fn desc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: Direction::Desc,
        }
    }
}
