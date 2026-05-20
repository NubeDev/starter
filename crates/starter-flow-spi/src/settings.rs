//! Node-settings schema + validation contracts.
//!
//! Per `DOCS/flow/scope/settings.md` Phase S-1: every
//! [`NodeBehavior`](crate::node::NodeBehavior) carries an associated
//! `Settings` type that derives `Deserialize` (for runtime
//! deserialisation) and `JsonSchema` (for publish-time validation and
//! editor form generation). The schema is *derived*, not
//! hand-written — there is no second artifact to keep in sync.
//!
//! This module provides:
//!
//! - [`SettingsError`] — the typed error a publish-time validator
//!   returns. Carries a JSON Pointer to the offending field so editor
//!   surfaces can highlight the wrong input without parsing free
//!   text (D-S6).
//! - [`EMPTY_SCHEMA`] — the schema returned by the
//!   [`NodeBehavior::config_schema`](crate::node::NodeBehavior::config_schema)
//!   default impl. A kind with no settings declares nothing and gets
//!   "any body is fine" semantics for free (D-S2).
//! - [`default_validate`] — the validator backing the
//!   [`NodeBehavior::validate_settings`](crate::node::NodeBehavior::validate_settings)
//!   default impl. Compiles the schema once per call into a
//!   `jsonschema` validator and reports the first violation as a
//!   structured [`SettingsError::SchemaViolation`].
//!
//! Runtime invocation is untouched — schema is a *publish-time gate*
//! (settings.md "What does NOT land"). Once
//! `DefinitionManager::publish` and `TopologyResolver::resolve` land
//! (see `DOCS/flow/scope/hot-reload.md`), the engine will call
//! [`NodeBehavior::validate_settings`](crate::node::NodeBehavior::validate_settings)
//! per node before writing a `FlowRevision`. Until then this module
//! is consumed only by external editors / smoke tests that fetch a
//! kind's schema directly.

use std::sync::LazyLock;

use schemars::schema::RootSchema;
use thiserror::Error;

/// JSON Schema document returned by the
/// [`NodeBehavior::config_schema`](crate::node::NodeBehavior::config_schema)
/// default impl. Accepts any JSON body; suitable for kinds that
/// declare no settings.
///
/// Per settings.md D-S2: the default impl lets every existing
/// `NodeBehavior` compile unchanged. Migration to typed settings is
/// opt-in per kind.
pub static EMPTY_SCHEMA: LazyLock<RootSchema> = LazyLock::new(RootSchema::default);

/// Errors surfaced when a draft's per-node `settings` object fails
/// validation against the kind's schema.
///
/// Per settings.md S-7: every variant carries enough machine-readable
/// context for an editor surface to point a user at the offending
/// field without parsing the [`Self::SchemaViolation::detail`] string.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SettingsError {
    /// The draft's JSON shape did not match the kind's schema.
    ///
    /// `pointer` is a JSON Pointer (RFC 6901) into the settings body
    /// at the offending field (e.g. `/cost_cap`). `rule` is the
    /// `jsonschema` keyword that failed (`"type"`, `"required"`,
    /// `"additionalProperties"`, …). `detail` is the English
    /// message the `jsonschema` crate produced, suitable for logs
    /// and fallback rendering.
    #[error("settings schema violation at `{pointer}` (rule `{rule}`): {detail}")]
    SchemaViolation {
        /// JSON Pointer into the settings body at the offending field.
        pointer: String,
        /// `jsonschema` keyword that failed.
        rule: &'static str,
        /// Human-readable description of the violation.
        detail: String,
    },

    /// The settings body could not be deserialised into the kind's
    /// `Settings` struct even after schema validation passed —
    /// indicates a schema / serde drift bug in the kind, not a user
    /// error. Carries the underlying `serde_json` error verbatim.
    #[error("settings deserialisation failed: {0}")]
    Deserialise(#[from] serde_json::Error),

    /// A kind-specific cross-field rule (returned by an overridden
    /// [`NodeBehavior::validate_settings`](crate::node::NodeBehavior::validate_settings))
    /// rejected the body. Example: *"if `auth_kind = bearer` then
    /// `auth_token` is required"* — a rule JSON Schema cannot
    /// express. `code` is a short stable machine-readable identifier;
    /// `detail` is human-friendly text.
    #[error("settings domain rule violation [{code}]: {detail}")]
    Domain {
        /// Short stable machine-readable code (kind-defined).
        code: &'static str,
        /// Human-readable description of the rule violation.
        detail: String,
    },
}

/// Default validator implementing
/// [`NodeBehavior::validate_settings`](crate::node::NodeBehavior::validate_settings).
///
/// Serialises the kind's [`RootSchema`] into JSON, compiles it with
/// the `jsonschema` crate, and runs the draft `body` through. The
/// first violation surfaces as [`SettingsError::SchemaViolation`]
/// with a JSON Pointer pinned at the offending field.
///
/// Per settings.md S-2 this is the default; kinds that need cross-
/// field rules JSON Schema cannot express override
/// [`NodeBehavior::validate_settings`](crate::node::NodeBehavior::validate_settings)
/// and return [`SettingsError::Domain`].
pub fn default_validate(
    schema: &RootSchema,
    body: &serde_json::Value,
) -> Result<(), SettingsError> {
    // Serialise the derived schema once per call. The `LazyLock`
    // wrapping in each kind keeps the `RootSchema` itself a one-shot
    // build; this `to_value` is cheap (the schema is small) and
    // avoids holding a global `JSONSchema` compiled view that would
    // need its own synchronisation.
    let schema_json = serde_json::to_value(schema).map_err(SettingsError::Deserialise)?;
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_json)
        .map_err(|e| SettingsError::SchemaViolation {
            pointer: String::new(),
            rule: "schema",
            detail: format!("could not compile node-kind schema: {e}"),
        })?;
    if let Err(errors) = compiled.validate(body) {
        // Surface the *first* error — matches the editor UX
        // (highlight one field at a time) and avoids accumulating an
        // unbounded number of cascading "additionalProperties" hits
        // when a typo at the top level cascades through.
        if let Some(err) = errors.into_iter().next() {
            return Err(SettingsError::SchemaViolation {
                pointer: err.instance_path.to_string(),
                rule: jsonschema_rule(&err.kind),
                detail: err.to_string(),
            });
        }
    }
    Ok(())
}

/// Map a `jsonschema::error::ValidationErrorKind` to the stable
/// keyword string the doc's S-7 contract promises. The `jsonschema`
/// crate's `Display` already carries the keyword in the message; this
/// pulls it out as `&'static str` so callers can match on it without
/// regex-scraping the detail string.
fn jsonschema_rule(kind: &jsonschema::error::ValidationErrorKind) -> &'static str {
    use jsonschema::error::ValidationErrorKind as K;
    match kind {
        K::AdditionalItems { .. } => "additionalItems",
        K::AdditionalProperties { .. } => "additionalProperties",
        K::AnyOf => "anyOf",
        K::BacktrackLimitExceeded { .. } => "backtrackLimit",
        K::Constant { .. } => "const",
        K::Contains => "contains",
        K::ContentEncoding { .. } => "contentEncoding",
        K::ContentMediaType { .. } => "contentMediaType",
        K::Enum { .. } => "enum",
        K::ExclusiveMaximum { .. } => "exclusiveMaximum",
        K::ExclusiveMinimum { .. } => "exclusiveMinimum",
        K::FalseSchema => "false",
        K::Format { .. } => "format",
        K::FromUtf8 { .. } => "utf8",
        K::MaxItems { .. } => "maxItems",
        K::Maximum { .. } => "maximum",
        K::MaxLength { .. } => "maxLength",
        K::MaxProperties { .. } => "maxProperties",
        K::MinItems { .. } => "minItems",
        K::Minimum { .. } => "minimum",
        K::MinLength { .. } => "minLength",
        K::MinProperties { .. } => "minProperties",
        K::MultipleOf { .. } => "multipleOf",
        K::Not { .. } => "not",
        K::OneOfMultipleValid => "oneOf",
        K::OneOfNotValid => "oneOf",
        K::Pattern { .. } => "pattern",
        K::PropertyNames { .. } => "propertyNames",
        K::Required { .. } => "required",
        K::Type { .. } => "type",
        K::UniqueItems => "uniqueItems",
        K::Custom { .. } => "custom",
        K::FileNotFound { .. } | K::JSONParse { .. } | K::InvalidURL { .. } | K::Resolver { .. } => {
            "resolver"
        }
        K::Schema => "schema",
        K::Utf8 { .. } => "utf8",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)] // fields exist only so the derived schema is non-trivial
    struct Demo {
        provider_id: String,
        #[serde(default)]
        cost_cap: f64,
    }

    fn demo_schema() -> RootSchema {
        schemars::schema_for!(Demo)
    }

    #[test]
    fn empty_schema_accepts_any_body() {
        let body = serde_json::json!({ "anything": [1, 2, 3] });
        default_validate(&EMPTY_SCHEMA, &body).expect("empty schema accepts any body");
    }

    #[test]
    fn happy_path_validates() {
        let schema = demo_schema();
        let body = serde_json::json!({ "provider_id": "anthropic/claude" });
        default_validate(&schema, &body).expect("body validates");
    }

    #[test]
    fn wrong_type_reports_pointer_and_rule() {
        let schema = demo_schema();
        let body = serde_json::json!({
            "provider_id": "anthropic/claude",
            "cost_cap": "banana"
        });
        let err = default_validate(&schema, &body).expect_err("must reject string for float");
        match err {
            SettingsError::SchemaViolation { pointer, rule, .. } => {
                assert_eq!(pointer, "/cost_cap", "JSON Pointer at the offending field");
                assert_eq!(rule, "type", "rule is the schema keyword");
            }
            other => panic!("expected SchemaViolation, got {other:?}"),
        }
    }

    #[test]
    fn missing_required_field_rejected() {
        let schema = demo_schema();
        let body = serde_json::json!({});
        let err = default_validate(&schema, &body).expect_err("must reject missing required");
        match err {
            SettingsError::SchemaViolation { rule, .. } => {
                assert_eq!(rule, "required");
            }
            other => panic!("expected SchemaViolation, got {other:?}"),
        }
    }

    #[test]
    fn unknown_field_rejected_under_deny_unknown_fields() {
        let schema = demo_schema();
        let body = serde_json::json!({
            "provider_id": "anthropic/claude",
            "typo_field": 42
        });
        let err = default_validate(&schema, &body).expect_err("must reject unknown field");
        match err {
            SettingsError::SchemaViolation { rule, .. } => {
                assert_eq!(rule, "additionalProperties");
            }
            other => panic!("expected SchemaViolation, got {other:?}"),
        }
    }
}
