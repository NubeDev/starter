//! [`ExtensionId`]: a reverse-DNS string identifying an extension.
//!
//! Per SCOPE.md **R4**: every identifier an extension contributes (tool ids,
//! panel ids, kind ids, route ids, …) must be the extension id or a dotted
//! descendant. The loader rejects contributions that escape that subtree;
//! reserved prefixes (`sys.*`, `starter.*`) belong to the host.

use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::str::FromStr;

/// Prefixes the host reserves and refuses to let an extension claim.
///
/// Kept here (not in a higher crate) so the loader, the SDK proc-macro, and
/// any adapter that mints synthetic ids all see the same list.
pub const RESERVED_PREFIXES: &[&str] = &["sys", "starter"];

/// A validated reverse-DNS extension identifier.
///
/// Construct via [`ExtensionId::new`] (or `parse`); the inner string is
/// guaranteed to:
///
/// - contain at least two dot-separated labels (`com.acme`, never `acme`);
/// - use only ASCII letters, digits, `-`, and `_` within labels;
/// - start each label with an ASCII letter;
/// - not begin with a reserved prefix (`sys.*`, `starter.*`).
///
/// Equality and hashing are byte-exact. Two ids that differ only in case are
/// **distinct**; the validator rejects uppercase to keep `com.Acme.foo` and
/// `com.acme.foo` from coexisting.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ExtensionId(String);

impl ExtensionId {
    /// Parse and validate a reverse-DNS string.
    pub fn new(raw: impl Into<String>) -> Result<Self, InvalidExtensionId> {
        let s = raw.into();
        validate(&s)?;
        Ok(Self(s))
    }

    /// Borrow the inner string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Move the inner string out.
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Return `true` if `child` is `self` or a dotted descendant of `self`
    /// (`com.acme.weather` owns `com.acme.weather.current` but not
    /// `com.acme.weatherly`). Used by the loader to enforce R4 namespace
    /// ownership on every contributed id.
    pub fn owns(&self, child: &str) -> bool {
        if child == self.0 {
            return true;
        }
        let prefix_len = self.0.len();
        child.len() > prefix_len
            && child.starts_with(&self.0)
            && child.as_bytes()[prefix_len] == b'.'
    }
}

impl fmt::Display for ExtensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ExtensionId {
    type Err = InvalidExtensionId;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned())
    }
}

impl AsRef<str> for ExtensionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ExtensionId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        ExtensionId::new(s).map_err(serde::de::Error::custom)
    }
}

/// Why an `ExtensionId` candidate string was rejected.
///
/// Carried inside [`crate::Error::Manifest`] when the failure surfaces to a
/// user; kept as its own enum so callers that validate ids in isolation
/// (e.g. an `SdkBuild` step in `starter-ext-sdk`) get a structured reason.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InvalidExtensionId {
    /// Fewer than two dot-separated labels (`acme` instead of `com.acme`).
    #[error("extension id must have at least two dot-separated labels: {0:?}")]
    TooShort(String),

    /// A label was empty, started with a non-letter, or contained an illegal
    /// character. R4 demands the id be safe to embed in tool ids, file
    /// paths, and HTTP route segments without escaping.
    #[error("extension id label {label:?} is not a valid reverse-DNS label (in id {full:?})")]
    BadLabel {
        /// The original id, kept for the error message.
        full: String,
        /// The first label that failed validation.
        label: String,
    },

    /// The id starts with a host-reserved prefix (`sys`, `starter`).
    #[error("extension id {full:?} begins with reserved prefix {prefix:?}")]
    ReservedPrefix {
        /// The original id.
        full: String,
        /// The reserved prefix that matched.
        prefix: &'static str,
    },
}

fn validate(s: &str) -> Result<(), InvalidExtensionId> {
    let labels: Vec<&str> = s.split('.').collect();
    if labels.len() < 2 {
        return Err(InvalidExtensionId::TooShort(s.to_owned()));
    }
    for label in &labels {
        if !is_valid_label(label) {
            return Err(InvalidExtensionId::BadLabel {
                full: s.to_owned(),
                label: (*label).to_owned(),
            });
        }
    }
    for prefix in RESERVED_PREFIXES {
        if labels[0] == *prefix {
            return Err(InvalidExtensionId::ReservedPrefix {
                full: s.to_owned(),
                prefix,
            });
        }
    }
    Ok(())
}

fn is_valid_label(label: &str) -> bool {
    let mut chars = label.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_minimal_two_label_id() {
        let id = ExtensionId::new("com.acme").unwrap();
        assert_eq!(id.as_str(), "com.acme");
    }

    #[test]
    fn accepts_deep_id() {
        ExtensionId::new("com.acme.weather.tools.current").unwrap();
    }

    #[test]
    fn rejects_single_label() {
        assert!(matches!(
            ExtensionId::new("acme").unwrap_err(),
            InvalidExtensionId::TooShort(_)
        ));
    }

    #[test]
    fn rejects_uppercase() {
        assert!(matches!(
            ExtensionId::new("com.Acme").unwrap_err(),
            InvalidExtensionId::BadLabel { .. }
        ));
    }

    #[test]
    fn rejects_leading_digit() {
        assert!(matches!(
            ExtensionId::new("com.1acme").unwrap_err(),
            InvalidExtensionId::BadLabel { .. }
        ));
    }

    #[test]
    fn rejects_reserved_prefix() {
        assert!(matches!(
            ExtensionId::new("sys.core").unwrap_err(),
            InvalidExtensionId::ReservedPrefix { prefix: "sys", .. }
        ));
        assert!(matches!(
            ExtensionId::new("starter.something").unwrap_err(),
            InvalidExtensionId::ReservedPrefix {
                prefix: "starter",
                ..
            }
        ));
    }

    #[test]
    fn owns_exact_and_descendant() {
        let id = ExtensionId::new("com.acme.weather").unwrap();
        assert!(id.owns("com.acme.weather"));
        assert!(id.owns("com.acme.weather.current"));
        assert!(id.owns("com.acme.weather.panel.detail"));
    }

    #[test]
    fn owns_rejects_sibling_and_prefix_substring() {
        let id = ExtensionId::new("com.acme.weather").unwrap();
        assert!(!id.owns("com.acme.weatherly")); // prefix-substring, not a dotted child
        assert!(!id.owns("com.acme.other"));
        assert!(!id.owns("com.other.weather"));
    }

    #[test]
    fn serde_round_trip() {
        let id = ExtensionId::new("com.acme.weather").unwrap();
        let s = serde_json::to_string(&id).unwrap();
        assert_eq!(s, "\"com.acme.weather\"");
        let back: ExtensionId = serde_json::from_str(&s).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn serde_rejects_invalid() {
        let r: Result<ExtensionId, _> = serde_json::from_str("\"sys.core\"");
        assert!(r.is_err());
    }
}
