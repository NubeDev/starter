//! Token-value validation. Lives in `spi` so every transport (REST
//! today, MCP or gRPC if they ever expose the editor) enforces the
//! same denylist.

use thiserror::Error;

use super::ThemeSaveInput;

/// Substrings forbidden in any token value.
///
/// Theme tokens are interpolated directly into the page's CSS
/// custom properties. A value containing `url(`, `@import`, `expression(`
/// or `javascript:` could exfiltrate data through font / image
/// loads, smuggle stylesheets past CSP, or run script in legacy
/// engines. The deny list matches what `DOCS/frontend/theme/README.md`
/// promises ("Out of scope" section).
const DENIED: &[&str] = &["url(", "@import", "expression(", "javascript:"];

/// Per-key validation failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("token {key:?} contains forbidden substring {fragment:?}")]
pub struct TokenValueError {
    /// Token name that failed validation.
    pub key: String,
    /// Substring that triggered the failure.
    pub fragment: String,
}

/// Reject one token value if it contains any denied substring.
///
/// The check is case-insensitive on ASCII (CSS keywords like `URL(`
/// and `JavaScript:` are equivalent to their lowercase forms).
pub fn validate_token_value(key: &str, value: &str) -> Result<(), TokenValueError> {
    let lowered = value.to_ascii_lowercase();
    for denied in DENIED {
        if lowered.contains(denied) {
            return Err(TokenValueError {
                key: key.to_string(),
                fragment: (*denied).to_string(),
            });
        }
    }
    Ok(())
}

/// Validate every token in both modes plus the shell fields of a
/// save payload. Returns the **first** offending entry. Callers
/// rendering RFC 7807 problem bodies surface `key` + `fragment` in
/// `detail`.
pub fn validate_save_input(input: &ThemeSaveInput) -> Result<(), TokenValueError> {
    for (k, v) in &input.theme_styles.light {
        validate_token_value(&format!("light.{k}"), v)?;
    }
    for (k, v) in &input.theme_styles.dark {
        validate_token_value(&format!("dark.{k}"), v)?;
    }
    validate_token_value("shell.nav_title", &input.shell.nav_title)?;
    for (i, f) in input.shell.hide_features.iter().enumerate() {
        validate_token_value(&format!("shell.hide_features[{i}]"), f)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_url() {
        let err = validate_token_value("primary", "url(http://evil)").unwrap_err();
        assert_eq!(err.fragment, "url(");
    }

    #[test]
    fn rejects_javascript_uppercase() {
        let err = validate_token_value("primary", "JavaScript:alert(1)").unwrap_err();
        assert_eq!(err.fragment, "javascript:");
    }

    #[test]
    fn accepts_oklch() {
        validate_token_value("primary", "oklch(0.55 0.22 257)").unwrap();
    }

    #[test]
    fn accepts_font_stack() {
        validate_token_value("font-sans", "ui-sans-serif, system-ui, sans-serif").unwrap();
    }
}
