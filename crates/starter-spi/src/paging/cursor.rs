//! Opaque cursor type. The contents are an implementation detail of
//! the store backend; clients treat it as a black-box string.

use serde::{Deserialize, Serialize};

/// Opaque pagination cursor. Round-trip the value the server returned
/// in `Page::next_cursor`; never construct one client-side.
///
/// Backends are free to encode whatever they need inside (typically
/// a base64-encoded tuple of `(sort_key, last_id)`); from the API
/// surface this is just a string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cursor(String);

impl Cursor {
    /// Wrap a backend-produced string as a cursor.
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// Borrow the underlying string for backend decoding.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
