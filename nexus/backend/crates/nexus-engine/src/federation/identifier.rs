//! Strict SQL-identifier validation for the federation provider's text path.
//!
//! The Postgres provider inserts the remote table name into SQL text (`FROM
//! "<table>"`) — it cannot be a bound parameter. That makes this the injection
//! guard on the federation read path. The grammar mirrors the sink side's
//! validator (one ASCII segment) so both text paths agree; it lives here rather
//! than reaching into the sink's private module so the two lanes stay separate.

use crate::core::{EngineError, EngineResult};

/// Accept a single unqualified SQL identifier (a table name) or reject it.
/// Grammar: one segment `[A-Za-z_][A-Za-z0-9_]*`, ASCII only — no dots, quoting,
/// spaces, operators, or parentheses. Rejection is an [`EngineError::Source`]
/// because federation validates on the read side.
pub fn validate_identifier(raw: &str) -> EngineResult<&str> {
    let mut chars = raw.chars();
    let first_ok = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_');
    if first_ok && chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(raw)
    } else {
        Err(EngineError::Source(format!(
            "rejected SQL identifier {raw:?}: must match [A-Za-z_][A-Za-z0-9_]*"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_identifiers() {
        for ok in ["t", "_t", "table1", "device_reading", "Col_2"] {
            assert!(validate_identifier(ok).is_ok(), "{ok} should pass");
        }
    }

    #[test]
    fn rejects_injection_shapes() {
        for bad in ["", "1col", "a b", "a;b", "a-b", "a.b", "\"a\"", "a)", "drop table x"] {
            assert!(validate_identifier(bad).is_err(), "{bad:?} should be rejected");
        }
    }
}
