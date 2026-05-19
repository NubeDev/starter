//! `MessageKey` — a reverse-DNS-style translation key, validated on
//! construction.
//!
//! Same validation friction `starter-flow-spi::KindId` applies (the
//! merged sibling job): empty / all-whitespace / non-printable bytes /
//! leading-or-trailing `.` / `..` are refused. Otherwise the inner
//! string is preserved verbatim so wire round-trips are identity.

use core::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::I18nError;

/// A reverse-DNS-style translation key, e.g. `"flow.error"` or
/// `"auth.token.expired"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String", into = "String")]
#[schema(value_type = String, example = "auth.token.expired")]
pub struct MessageKey(String);

impl MessageKey {
    /// Parse and validate a message key. The rules:
    ///
    /// - non-empty after trimming;
    /// - every byte is printable ASCII (no control chars, no
    ///   whitespace anywhere inside);
    /// - does not start or end with `.`;
    /// - does not contain `..`.
    pub fn parse(input: impl Into<String>) -> Result<Self, I18nError> {
        let input = input.into();

        if input.is_empty() {
            return Err(I18nError::InvalidMessageKey {
                input,
                reason: "empty",
            });
        }
        if input.trim().is_empty() {
            return Err(I18nError::InvalidMessageKey {
                input,
                reason: "whitespace only",
            });
        }
        if input.chars().any(|c| c.is_whitespace()) {
            return Err(I18nError::InvalidMessageKey {
                input,
                reason: "contains whitespace",
            });
        }
        if input.chars().any(|c| c.is_control() || !c.is_ascii_graphic()) {
            return Err(I18nError::InvalidMessageKey {
                input,
                reason: "contains non-printable or non-ASCII byte",
            });
        }
        if input.starts_with('.') {
            return Err(I18nError::InvalidMessageKey {
                input,
                reason: "leading dot",
            });
        }
        if input.ends_with('.') {
            return Err(I18nError::InvalidMessageKey {
                input,
                reason: "trailing dot",
            });
        }
        if input.contains("..") {
            return Err(I18nError::InvalidMessageKey {
                input,
                reason: "double dot",
            });
        }

        Ok(Self(input))
    }

    /// Borrow the inner key string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the newtype and return the inner `String`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for MessageKey {
    type Err = I18nError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s.to_owned())
    }
}

impl TryFrom<String> for MessageKey {
    type Error = I18nError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<MessageKey> for String {
    fn from(value: MessageKey) -> Self {
        value.0
    }
}

impl core::fmt::Display for MessageKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}
