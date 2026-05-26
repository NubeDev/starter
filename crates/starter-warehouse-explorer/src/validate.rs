//! Identifier validation. Path parameters in `/tables/{name}` are
//! passed unquoted into SQL strings, so the only safe gate is to
//! reject everything that doesn't match
//! `^[A-Za-z_][A-Za-z0-9_]*$` *before* the value reaches a query.
//!
//! No schema-qualified names — `public` is implicit. No quoting,
//! no escaping, no Unicode identifiers. If you need any of those,
//! widen the regex here and audit every call-site.

/// True iff `name` is a safe-to-interpolate Postgres identifier.
///
/// ```text
/// ident := [A-Za-z_] [A-Za-z0-9_]*
/// ```
pub fn is_safe_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_simple_names() {
        assert!(is_safe_identifier("samples"));
        assert!(is_safe_identifier("_internal"));
        assert!(is_safe_identifier("raw_events_2026"));
        assert!(is_safe_identifier("A"));
    }

    #[test]
    fn rejects_empty() {
        assert!(!is_safe_identifier(""));
    }

    #[test]
    fn rejects_leading_digit() {
        assert!(!is_safe_identifier("2026_events"));
    }

    #[test]
    fn rejects_schema_qualified() {
        assert!(!is_safe_identifier("public.samples"));
    }

    #[test]
    fn rejects_quotes_and_semicolons() {
        assert!(!is_safe_identifier("samples; DROP TABLE"));
        assert!(!is_safe_identifier("\"samples\""));
        assert!(!is_safe_identifier("samples\""));
    }

    #[test]
    fn rejects_whitespace() {
        assert!(!is_safe_identifier("samples raw"));
        assert!(!is_safe_identifier(" samples"));
    }

    #[test]
    fn rejects_unicode() {
        assert!(!is_safe_identifier("sämples"));
    }
}
