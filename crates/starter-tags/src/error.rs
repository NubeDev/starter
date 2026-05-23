//! Typed errors for the tag crate.

use thiserror::Error;

/// Errors raised when parsing a [`crate::query::TagQuery`] from text.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TagParseError {
    #[error("empty query")]
    Empty,
    #[error("unexpected trailing input at byte {offset}: {tail:?}")]
    Trailing { offset: usize, tail: String },
    #[error(
        "float literal `{literal}` is not allowed in TagQuery (T7). \
         Numeric measurements belong in typed columns (e.g. samples.value_num); \
         numeric discriminants must be quoted strings."
    )]
    FloatLiteral { literal: String },
    #[error("syntax error near {near:?}")]
    Syntax { near: String },
}

/// Errors raised when constructing or mutating a [`crate::set::TagSet`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TagSetError {
    #[error(
        "tag value for key `{key}` is the reserved string {value:?}; \
         use TagValue::Bool to express booleans (T2 / M-2)"
    )]
    ReservedBoolString { key: String, value: String },
    #[error(
        "tag value for key `{key}` was a non-integer JSON number ({value}); \
         numeric measurements belong in typed columns (e.g. samples.value_num)"
    )]
    NonIntegerNumber { key: String, value: String },
    #[error(
        "tag value for key `{key}` was a non-finite JSON number; \
         NaN/Infinity are rejected at TagSet construction"
    )]
    NonFiniteNumber { key: String },
    #[error("tag key must be non-empty")]
    EmptyKey,
}
