//! DDL generators. Catalog rows in Postgres are the artefact of
//! record; the ClickHouse DDL emitted from them is the
//! implementation detail. Identifiers are validated against
//! `^[a-z][a-z0-9_]{0,62}$` before they touch any SQL string —
//! user input never reaches the SQL raw (W5).

pub mod cleaner;
pub mod mart;
pub mod sandbox;

/// Validate an identifier (mart name, sandbox name, column name,
/// `group_by` key, aggregation alias). Returns the input on success
/// so the caller can keep flow.
pub fn validate_ident(s: &str) -> Result<&str, IdentError> {
    if s.is_empty() || s.len() > 63 {
        return Err(IdentError(s.to_string()));
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_lowercase()) {
        return Err(IdentError(s.to_string()));
    }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(IdentError(s.to_string()));
        }
    }
    Ok(s)
}

#[derive(Debug, thiserror::Error)]
#[error("invalid identifier: {0:?}")]
pub struct IdentError(pub String);
