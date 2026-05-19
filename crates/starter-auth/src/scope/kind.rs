//! `Scope` — opaque string. Consumers define their own scope
//! vocabulary; starter-auth only enforces "principal must have
//! scope X" at the middleware boundary.

use serde::{Deserialize, Serialize};

/// A named permission scope.
///
/// Scope strings are consumer-defined and stored opaque. Convention:
/// `verb:resource` (e.g. `read:metrics`, `write:flows`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Scope(pub String);

impl Scope {
    /// Wrap a string as a scope.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}
