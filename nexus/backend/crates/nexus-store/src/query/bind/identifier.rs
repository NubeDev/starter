//! Validate the one text path the binder cannot bind: SQL identifiers.
//!
//! Values are always bound as `$N` args, but a *column name* (the `col` in
//! `$__timeFilter(col)`) cannot be a bind parameter in SQL — it must be inserted
//! as text. That makes identifier validation the single injection guard on the
//! text path, so it is strict by construction: an allowlisted shape, never an
//! escape-and-hope. See docs/design/query/.

use super::error::BindError;

/// Accept a (optionally schema- or table-qualified) Postgres identifier and
/// return it verbatim, or reject it. The grammar is deliberately narrow:
///
/// - one to three dot-separated segments (`col`, `t.col`, `schema.t.col`);
/// - each segment starts with a letter or underscore, then letters, digits, or
///   underscores;
/// - no quoting, no spaces, no operators, no parentheses.
///
/// Anything outside this shape — including a quoted identifier or one with
/// punctuation that could break out of the intended position — is rejected.
/// Legitimate quoted/mixed-case identifiers are out of scope for the macro
/// argument slot; the author can still reference them in the surrounding raw
/// SQL, which is the existing guarded path.
pub fn validate_identifier(raw: &str) -> Result<String, BindError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(BindError::InvalidIdentifier(raw.to_string()));
    }
    let segments: Vec<&str> = trimmed.split('.').collect();
    if segments.is_empty() || segments.len() > 3 {
        return Err(BindError::InvalidIdentifier(raw.to_string()));
    }
    for segment in &segments {
        if !is_valid_segment(segment) {
            return Err(BindError::InvalidIdentifier(raw.to_string()));
        }
    }
    Ok(trimmed.to_string())
}

/// One identifier segment: `[A-Za-z_][A-Za-z0-9_]*`. ASCII-only on purpose —
/// the bucketing/time columns this guards are catalog identifiers, and widening
/// to Unicode would widen the one text-insertion surface for no real gain.
fn is_valid_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
