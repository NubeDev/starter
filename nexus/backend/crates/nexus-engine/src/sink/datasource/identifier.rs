//! Strict SQL-identifier validation for the datasource sink's text path.
//!
//! A datasource write inserts the target table name and the JSON-derived column
//! names into SQL text — neither can be a bound parameter. That makes identifier
//! validation the single injection guard on the write path, so it is strict by
//! construction: an allowlisted shape, never escape-and-hope. A device payload
//! key (which becomes a column name) may therefore never reach SQL text
//! unvalidated. The grammar mirrors the query binder's `validate_identifier`
//! (the established text-path precedent) so the two guards agree.

use crate::core::{EngineError, EngineResult};

/// Accept a single unqualified SQL identifier — a table or column name — and
/// return it, or reject it. The grammar is deliberately narrow:
///
/// - one segment, `[A-Za-z_][A-Za-z0-9_]*` (ASCII only);
/// - no dots, quoting, spaces, operators, or parentheses.
///
/// Anything outside this shape is rejected with [`EngineError::Sink`]. A table
/// name validates identically to a column name: the sink never qualifies the
/// table (the datasource record selects the database), so a single-segment
/// grammar is the whole surface.
pub fn validate_identifier(raw: &str) -> EngineResult<&str> {
    let mut chars = raw.chars();
    let first_ok = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_');
    if first_ok && chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(raw)
    } else {
        Err(EngineError::Sink(format!(
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
        for bad in [
            "",
            "1col",
            "a b",
            "a;b",
            "a-b",
            "a.b",
            "\"a\"",
            "a)",
            "drop table x",
            "a--",
        ] {
            assert!(
                validate_identifier(bad).is_err(),
                "{bad:?} should be rejected"
            );
        }
    }
}
