//! Catalog format + loader. See SCOPE.md Phase 3.
//!
//! # Format
//!
//! A catalog is a plain JSON object keyed by
//! [`starter_spi::i18n::MessageKey`], with ICU MessageFormat string
//! values. The wire shape is intentionally flat:
//!
//! ```json
//! {
//!   "auth.token.expired": "Your session has expired.",
//!   "settings.units.metric": "Metric (°C, kPa, km)"
//! }
//! ```
//!
//! No nesting, no namespacing-by-object, no `"messages": { … }`
//! wrapper. The flat shape keeps the loader trivial and the wire
//! diffable.
//!
//! # `deny_unknown_fields` posture
//!
//! `MessageKey::parse` is the gate. Every top-level key in the JSON
//! object MUST satisfy [`MessageKey`] validation — empty strings,
//! leading / trailing / doubled dots, whitespace, or control
//! characters all fail. This is the "deny unknown top-level keys"
//! lock from the SCOPE Phase 3 "Decisions" block: any key that is
//! not a valid `MessageKey` is rejected fast at parse time, not
//! silently coerced or skipped. Values must be JSON strings; a
//! number / array / object value fails the parse.
//!
//! # Fingerprint
//!
//! [`Catalog::fingerprint`] is the 16-char lowercase-hex prefix of
//! the sha256 of the catalog's canonical JSON serialisation
//! (`BTreeMap` ordering, no extra whitespace). The fingerprint is
//! used by `GET /v1/i18n/manifest` to produce immutable
//! content-addressed URLs; the 16-char prefix is plenty for
//! collision avoidance across a single product's catalogs.
//!
//! # Loading
//!
//! Two entry points cover the SCOPE-required surface:
//!
//! - [`Catalog::from_json_str`] — parse any in-memory JSON string.
//!   Use this for compiled-in `const &str` chrome catalogs
//!   (`include_str!("…/en.json")` in [`crate::platform`]) so a
//!   binary serves starter chrome even when no on-disk catalog dir
//!   exists.
//! - [`Catalog::from_file`] — read a path off disk. Use this for
//!   product-owned catalogs supplied at deploy time.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use starter_spi::i18n::MessageKey;

/// Errors the catalog loader can raise.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// I/O failure while reading a catalog file from disk.
    #[error("failed to read catalog at {path}: {source}")]
    Io {
        /// Path that was being read when the error surfaced.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The bytes were not valid JSON, the JSON shape did not match
    /// the catalog format (flat `{ MessageKey: string }`), or a
    /// top-level key failed [`MessageKey`] validation.
    #[error("failed to parse catalog: {0}")]
    Parse(#[from] serde_json::Error),
}

/// An in-memory snapshot of a single language's catalog.
///
/// Serialises transparently as a flat JSON object of
/// `MessageKey → ICU MessageFormat string` (see module docs).
/// Because the inner map is a [`BTreeMap`], iteration and
/// serialisation are deterministic — this is what makes
/// [`Catalog::fingerprint`] reproducible across runs.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Catalog {
    /// Flat map of message key → ICU MessageFormat string.
    pub messages: BTreeMap<MessageKey, String>,
}

impl Catalog {
    /// Build a catalog from an in-memory string of JSON.
    ///
    /// Intended for the compiled-in `const &str` pattern in
    /// [`crate::platform`] — see module docs.
    pub fn from_json_str(s: &str) -> Result<Self, CatalogError> {
        let cat: Self = serde_json::from_str(s)?;
        Ok(cat)
    }

    /// Read a catalog from disk.
    ///
    /// The file is expected to be UTF-8 JSON in the
    /// [module-level format](self).
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CatalogError> {
        let path_ref = path.as_ref();
        let bytes = std::fs::read(path_ref).map_err(|source| CatalogError::Io {
            path: path_ref.display().to_string(),
            source,
        })?;
        let cat: Self = serde_json::from_slice(&bytes)?;
        Ok(cat)
    }

    /// Number of messages in this catalog.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// `true` if this catalog carries no messages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get the ICU MessageFormat string for `key`, if present.
    #[must_use]
    pub fn get(&self, key: &MessageKey) -> Option<&str> {
        self.messages.get(key).map(String::as_str)
    }

    /// 16-char lowercase-hex prefix of the sha256 of the catalog's
    /// canonical JSON serialisation.
    ///
    /// Deterministic for a given set of `(key, value)` pairs because
    /// the inner [`BTreeMap`] iterates in sorted order and
    /// `serde_json::to_vec` emits no extra whitespace.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        let canonical = serde_json::to_vec(self).expect("Catalog always serialises");
        let mut hasher = Sha256::new();
        hasher.update(&canonical);
        let digest = hasher.finalize();
        let hex = format!("{digest:x}");
        hex[..16].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> MessageKey {
        MessageKey::parse(s).expect("test key must parse")
    }

    #[test]
    fn round_trips_a_valid_catalog() {
        let json = r#"{
            "auth.token.expired": "Your session has expired.",
            "settings.units.metric": "Metric"
        }"#;
        let cat = Catalog::from_json_str(json).expect("valid catalog");
        assert_eq!(cat.len(), 2);
        assert_eq!(
            cat.get(&key("auth.token.expired")),
            Some("Your session has expired."),
        );
        assert_eq!(cat.get(&key("settings.units.metric")), Some("Metric"));

        let back = serde_json::to_string(&cat).unwrap();
        assert!(back.starts_with('{'));
        assert!(back.contains("\"auth.token.expired\""));
        assert!(back.contains("\"settings.units.metric\""));
    }

    #[test]
    fn unknown_top_level_key_fails_fast() {
        // Leading-dot key is not a valid MessageKey; parsing must
        // fail rather than silently store an unreachable key.
        let json = r#"{ ".bad.key": "x" }"#;
        let err = Catalog::from_json_str(json).expect_err("leading-dot key rejected");
        assert!(matches!(err, CatalogError::Parse(_)));

        let json = r#"{ "flow..error": "x" }"#;
        let err = Catalog::from_json_str(json).expect_err("doubled-dot key rejected");
        assert!(matches!(err, CatalogError::Parse(_)));

        let json = r#"{ "": "x" }"#;
        let err = Catalog::from_json_str(json).expect_err("empty key rejected");
        assert!(matches!(err, CatalogError::Parse(_)));
    }

    #[test]
    fn non_string_value_fails_fast() {
        let json = r#"{ "auth.ok": 42 }"#;
        let err = Catalog::from_json_str(json).expect_err("number value must fail");
        assert!(matches!(err, CatalogError::Parse(_)));
    }

    #[test]
    fn malformed_json_fails_fast() {
        let err = Catalog::from_json_str("{not json").expect_err("garbage must fail");
        assert!(matches!(err, CatalogError::Parse(_)));
    }

    #[test]
    fn fingerprint_is_stable_and_16_chars() {
        let cat = Catalog::from_json_str(r#"{ "a.b": "one", "c.d": "two" }"#).unwrap();
        let fp1 = cat.fingerprint();
        let fp2 = cat.fingerprint();
        assert_eq!(fp1, fp2);
        assert_eq!(fp1.len(), 16);
        assert!(fp1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn fingerprint_is_insensitive_to_input_key_order() {
        let a = Catalog::from_json_str(r#"{ "a.b": "1", "c.d": "2" }"#).unwrap();
        let b = Catalog::from_json_str(r#"{ "c.d": "2", "a.b": "1" }"#).unwrap();
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_a_value_changes() {
        let a = Catalog::from_json_str(r#"{ "a.b": "one" }"#).unwrap();
        let b = Catalog::from_json_str(r#"{ "a.b": "two" }"#).unwrap();
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn from_file_round_trips_from_disk() {
        let dir = TempDirGuard::new();
        let path = dir.path().join("en.json");
        std::fs::write(&path, r#"{ "ok.key": "yes" }"#).unwrap();
        let cat = Catalog::from_file(&path).expect("disk catalog");
        assert_eq!(cat.get(&key("ok.key")), Some("yes"));
    }

    #[test]
    fn from_file_missing_path_reports_io_error() {
        let err = Catalog::from_file("/definitely/does/not/exist/i18n.json")
            .expect_err("missing path must error");
        assert!(matches!(err, CatalogError::Io { .. }));
    }

    /// Hand-rolled temp dir under `std::env::temp_dir()` —
    /// `tempfile` is not currently a workspace dep and this
    /// keeps the dep posture clean.
    struct TempDirGuard {
        path: std::path::PathBuf,
    }
    impl TempDirGuard {
        fn new() -> Self {
            let unique = format!(
                "starter-i18n-catalog-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0),
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("temp dir creatable");
            Self { path }
        }
        fn path(&self) -> &Path {
            &self.path
        }
    }
    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
