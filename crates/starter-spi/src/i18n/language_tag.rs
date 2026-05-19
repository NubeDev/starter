//! `LanguageTag` — a BCP-47 string validated on construction.
//!
//! Per the SCOPE Library-choices table: `starter-spi` depends on
//! `icu_locale_core` so this newtype borrows the canonical BCP-47
//! parser instead of hand-rolling a regex. The parser refuses
//! `"en_US"` (underscore is not a BCP-47 separator), empty strings,
//! and arbitrary free text — see this crate's `tests.rs` for the
//! pinned positives and negatives.

use core::str::FromStr;

use icu_locale_core::LanguageIdentifier;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::I18nError;

/// A BCP-47 language tag, validated on construction.
///
/// The wire form is the raw tag string (e.g. `"en-US"`); the inner
/// value is preserved verbatim so a round-trip through serde returns
/// the same bytes the producer wrote.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String", into = "String")]
#[schema(value_type = String, example = "en-US")]
pub struct LanguageTag(String);

impl LanguageTag {
    /// Parse and validate a BCP-47 language tag. Returns
    /// [`I18nError::InvalidLanguageTag`] if `icu_locale_core` rejects
    /// the input.
    pub fn parse(input: impl Into<String>) -> Result<Self, I18nError> {
        let input = input.into();
        match LanguageIdentifier::try_from_str(&input) {
            Ok(_) => Ok(Self(input)),
            Err(err) => Err(I18nError::InvalidLanguageTag {
                input,
                reason: err.to_string(),
            }),
        }
    }

    /// Borrow the inner tag string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the newtype and return the inner `String`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for LanguageTag {
    type Err = I18nError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s.to_owned())
    }
}

impl TryFrom<String> for LanguageTag {
    type Error = I18nError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<LanguageTag> for String {
    fn from(value: LanguageTag) -> Self {
        value.0
    }
}

impl core::fmt::Display for LanguageTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}
