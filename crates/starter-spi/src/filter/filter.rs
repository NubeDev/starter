//! `Filter` — a list of `Predicate`s combined with implicit AND.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::predicate::Predicate;

/// A composed filter: every predicate must hold for a row to match.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct Filter {
    /// Conjunction of predicates. An empty list matches everything.
    pub predicates: Vec<Predicate>,
}

impl Filter {
    /// Build an empty filter (matches everything).
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a predicate, returning the filter for chaining.
    pub fn and(mut self, predicate: Predicate) -> Self {
        self.predicates.push(predicate);
        self
    }
}
