//! `Id<T>` — phantom-typed UUID wrapper.

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A typed identifier. The phantom `T` carries the resource type
/// so the type system catches "passed a project id where a user id
/// was expected".
///
/// The underlying representation is a v4 UUID. Serialization is the
/// raw UUID string — the phantom does not appear in JSON.
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<T> {
    value: Uuid,
    #[serde(skip)]
    _marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Generate a new random v4 id.
    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4(),
            _marker: PhantomData,
        }
    }

    /// Wrap an existing UUID as a typed id. Use sparingly — prefer
    /// `new()` so callers can't fabricate phantom-typed mismatches.
    pub fn from_uuid(value: Uuid) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }

    /// Borrow the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.value
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Id<T> {}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}
