//! Structured server→client messages — the payload of
//! [`crate::ActionResponse::Diagnostics`].
//!
//! Wire shape: `{ severity, code, message, field? }`. `field` is
//! omitted (not emitted as `""`) for global / form-level items per
//! the SDUI scope's R5 contract.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Severity for a [`Diagnostic`] — the renderer maps these to its
/// banner / inline-error / info-pill styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Hard failure — the request did not produce the intended
    /// effect. Renderer surfaces inline-by-field when `field` is set,
    /// page-level otherwise.
    Error,
    /// The request succeeded but the server wants the user to notice
    /// something (deprecation, partial result, near-limit value).
    Warning,
    /// Purely informational; never blocks a form submit.
    Info,
}

/// One item in an [`crate::ActionResponse::Diagnostics`] payload.
///
/// Replaces Rubix's per-field `form_errors` map (see
/// [`DOCS/frontend/sdui/DIVERGENCE.md`] D1). The wider shape covers
/// warnings and info, not just per-field errors, and the flat list
/// with an optional `field` anchor covers both global and inline
/// cases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Stable, dot-separated identifier; first segment matches the
    /// I18N catalog namespace.
    pub code: String,
    /// Server-translated human message. Clients render verbatim.
    pub message: String,
    /// Form-field anchor. Omit (do not send `""`) for a global
    /// diagnostic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Optional code-specific structured data (e.g. `{ "max": 64 }`).
    /// Renderers only read keys they understand for a known `code`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<JsonValue>,
}

impl Diagnostic {
    /// True when no `field` is set — renders as a form-level banner.
    pub fn is_global(&self) -> bool {
        self.field.is_none()
    }

    /// Convenience constructor for the most common shape.
    pub fn new(severity: Severity, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            field: None,
            details: None,
        }
    }

    /// Anchor the diagnostic to a form field.
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Attach a structured details payload.
    pub fn with_details(mut self, details: JsonValue) -> Self {
        self.details = Some(details);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn field_omitted_when_none() {
        let d = Diagnostic::new(Severity::Error, "x.y", "msg");
        let v = serde_json::to_value(&d).unwrap();
        assert!(
            v.get("field").is_none(),
            "global diagnostic must omit `field` key"
        );
        assert!(d.is_global());
    }

    #[test]
    fn field_present_when_set() {
        let d = Diagnostic::new(Severity::Error, "x.y", "msg").with_field("projectCode");
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["field"], "projectCode");
        assert!(!d.is_global());
    }

    #[test]
    fn round_trip_with_details() {
        let d = Diagnostic::new(Severity::Warning, "task.due.soon", "Soon")
            .with_field("dueDate")
            .with_details(json!({ "hours": 4 }));
        let s = serde_json::to_string(&d).unwrap();
        let back: Diagnostic = serde_json::from_str(&s).unwrap();
        assert_eq!(back, d);
    }
}
