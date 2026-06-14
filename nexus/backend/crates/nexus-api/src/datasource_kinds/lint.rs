//! Boot-time lints over a datasource-kind declaration.
//!
//! A failure aborts startup so a malformed connector declaration never ships:
//!
//! 1. **Object-schema lint:** the config schema must be an object schema with a
//!    `properties` map — a config form and the secret-field check both read it.
//! 2. **Secret-field lint:** every declared `secret_field` must be a property the
//!    config schema declares. A typo'd secret field would silently leave a
//!    credential unsealed, so it fails to load instead.

use super::error::DatasourceKindError;
use super::kind::DatasourceKind;

/// Run every load-time lint over `kind`. Returns the first failure.
pub fn check(kind: &DatasourceKind) -> Result<(), DatasourceKindError> {
    check_object_schema(kind)?;
    check_secret_fields(kind)?;
    Ok(())
}

/// The config schema must declare a `properties` object; without it there are no
/// fields to render a form from or to seal as secrets.
fn check_object_schema(kind: &DatasourceKind) -> Result<(), DatasourceKindError> {
    let has_props = kind
        .config_schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .is_some();
    if has_props {
        return Ok(());
    }
    Err(DatasourceKindError::Lint {
        kind: kind.name.clone(),
        detail: "config_schema must be an object schema with a `properties` map".to_string(),
    })
}

/// Every declared secret field must be a property the config schema declares, so
/// the seal/redact/decrypt boundary covers exactly the fields that exist.
fn check_secret_fields(kind: &DatasourceKind) -> Result<(), DatasourceKindError> {
    let declared = kind.declared_config_fields();
    for field in &kind.secret_fields {
        if !declared.contains(field) {
            return Err(DatasourceKindError::Lint {
                kind: kind.name.clone(),
                detail: format!(
                    "secret_field `{field}` is not a property of the config schema {declared:?}"
                ),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::manifest::{Surface, TestSpec};
    use super::*;
    use serde_json::json;

    fn kind(schema: serde_json::Value, secrets: &[&str]) -> DatasourceKind {
        DatasourceKind {
            name: "test".to_string(),
            surface: Surface::Query,
            config_schema: schema,
            secret_fields: secrets.iter().map(|s| (*s).to_string()).collect(),
            test: TestSpec::Connect,
            dialect: None,
            description: None,
        }
    }

    #[test]
    fn accepts_secret_field_that_is_a_declared_property() {
        let k = kind(
            json!({ "type": "object", "properties": { "password": { "type": "string" } } }),
            &["password"],
        );
        check(&k).expect("a declared secret property passes");
    }

    #[test]
    fn rejects_secret_field_absent_from_schema() {
        let k = kind(
            json!({ "type": "object", "properties": { "host": { "type": "string" } } }),
            &["passwrd"],
        );
        let err = check(&k).expect_err("a typo'd secret field must fail the lint");
        assert!(matches!(err, DatasourceKindError::Lint { .. }));
        assert!(err.to_string().contains("passwrd"));
    }

    #[test]
    fn rejects_schema_without_properties() {
        let k = kind(json!({ "type": "string" }), &[]);
        let err = check(&k).expect_err("a non-object config schema must fail the lint");
        assert!(matches!(err, DatasourceKindError::Lint { .. }));
    }
}
