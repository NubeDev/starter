//! `Scope` — opaque permission string attached to tokens.
//!
//! Consumers define their own scope vocabulary; the auth layer only
//! enforces "principal must have scope X" at middleware boundaries.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A named permission scope.
///
/// Convention: `verb:resource` (e.g. `read:metrics`, `write:flows`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct Scope(pub String);

impl Scope {
    /// Wrap a string as a scope.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
