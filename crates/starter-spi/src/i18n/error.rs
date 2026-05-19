//! Errors emitted by the `i18n` module surface.

use thiserror::Error;

/// Failure modes for constructing the validated i18n newtypes.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum I18nError {
    /// The string failed BCP-47 parsing by `icu_locale_core`. Most
    /// commonly: underscore separator (`"en_US"`), empty input, or a
    /// non-language-tag like `"not a tag"`.
    #[error("invalid BCP-47 language tag: {input:?} ({reason})")]
    InvalidLanguageTag {
        /// The string the caller passed in.
        input: String,
        /// A short human-readable explanation from the parser.
        reason: String,
    },

    /// The string failed `MessageKey` validation. Empty, all-whitespace,
    /// contained a non-printable byte, started or ended with a `.`, or
    /// contained `..`.
    #[error("invalid MessageKey: {input:?} ({reason})")]
    InvalidMessageKey {
        /// The string the caller passed in.
        input: String,
        /// Which validation rule rejected it.
        reason: &'static str,
    },
}
