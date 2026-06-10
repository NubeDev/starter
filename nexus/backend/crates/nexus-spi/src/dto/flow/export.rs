//! The portable flow JSON model — share a saved flow as a file.
//!
//! `GET /api/v1/flows/:id/export` emits a [`FlowExport`]; `POST
//! /api/v1/flows/import` validates `schema_version` and re-creates from one. The
//! shape is self-contained — the ArkFlow `{input, pipeline, output}` config plus
//! a name — so an exported flow is portable across tenants. The server-minted
//! `id` and the live `enabled`/run state are intentionally absent: an import
//! always mints a fresh id and lands the flow stopped (`enabled = false`) so a
//! shared file never auto-starts on someone else's node.
//!
//! Secrets are stripped on export. A flow's input/output config can embed
//! credentials inline (a Postgres `uri` with a password, an API key), which must
//! not travel in a file a user shares. [`redact_secrets`] blanks well-known
//! secret-bearing keys; the importer re-enters them. `redacted` records whether
//! anything was blanked so the UI can warn on both ends.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The current flow-model schema version. Bumped only on a breaking change; an
/// import rejects a version it does not understand.
pub const FLOW_SCHEMA_VERSION: u32 = 1;

/// A self-contained, importable flow: its name plus the ArkFlow config blobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FlowExport {
    /// Model version; must match [`FLOW_SCHEMA_VERSION`] to import.
    pub schema_version: u32,
    pub name: String,
    /// ArkFlow input component (`{type, ...config}`), secrets redacted.
    pub input: Value,
    /// ArkFlow processor list (a JSON array of `{type, ...config}`).
    #[serde(default)]
    pub pipeline: Value,
    /// ArkFlow output component (`{type, ...config}`), secrets redacted.
    pub output: Value,
    /// Whether [`redact_secrets`] blanked any field on export, so the UI can
    /// tell the user credentials were removed (and must be re-entered on
    /// import). Skipped when false so a clean export stays minimal.
    #[serde(default, skip_serializing_if = "is_false")]
    pub redacted: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Object keys whose values are treated as secrets and blanked on export. Match
/// is case-insensitive on the exact key (not a substring) so an innocuous
/// `secret_count` is not caught while `api_key`/`apiKey` is.
const SECRET_KEYS: [&str; 7] = [
    "uri",
    "url",
    "password",
    "token",
    "api_key",
    "apikey",
    "secret",
];

/// Recursively blank the values of any [`SECRET_KEYS`] found in `value`,
/// returning whether anything was redacted. Strings become `""`; the key is
/// kept so the shape round-trips and the importer sees the field to fill.
/// Non-secret keys recurse so a credential nested in a sub-object is still
/// caught.
pub fn redact_secrets(value: &mut Value) -> bool {
    let mut redacted = false;
    match value {
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                if SECRET_KEYS.contains(&k.to_ascii_lowercase().as_str()) {
                    // Blank in place, preserving the JSON type so an importer's
                    // form still reads the field as the right kind.
                    if !is_blank(v) {
                        *v = blank_like(v);
                        redacted = true;
                    }
                } else {
                    redacted |= redact_secrets(v);
                }
            }
        }
        Value::Array(items) => {
            for v in items.iter_mut() {
                redacted |= redact_secrets(v);
            }
        }
        _ => {}
    }
    redacted
}

/// Whether a value is already empty (so redaction is a no-op and we should not
/// claim we redacted anything).
fn is_blank(v: &Value) -> bool {
    match v {
        Value::String(s) => s.is_empty(),
        Value::Null => true,
        _ => false,
    }
}

/// An empty value of the same JSON kind, so blanking a string yields `""` and
/// blanking anything else yields `null`.
fn blank_like(v: &Value) -> Value {
    match v {
        Value::String(_) => Value::String(String::new()),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn blanks_top_level_secret_keys_and_keeps_the_rest() {
        let mut v = json!({
            "type": "postgres",
            "uri": "postgres://user:p4ss@host/db",
            "table": "readings",
        });
        let redacted = redact_secrets(&mut v);
        assert!(redacted);
        assert_eq!(v["uri"], json!(""));
        assert_eq!(v["table"], json!("readings"));
        assert_eq!(v["type"], json!("postgres"));
    }

    #[test]
    fn matches_keys_case_insensitively_and_blanks_to_same_kind() {
        let mut v = json!({ "ApiKey": "abc", "Token": 12345 });
        assert!(redact_secrets(&mut v));
        // String secret → "", non-string secret → null.
        assert_eq!(v["ApiKey"], json!(""));
        assert_eq!(v["Token"], Value::Null);
    }

    #[test]
    fn recurses_into_nested_objects_and_arrays() {
        let mut v = json!({
            "auth": { "password": "hunter2" },
            "endpoints": [{ "url": "https://x/y" }, { "name": "ok" }],
        });
        assert!(redact_secrets(&mut v));
        assert_eq!(v["auth"]["password"], json!(""));
        assert_eq!(v["endpoints"][0]["url"], json!(""));
        assert_eq!(v["endpoints"][1]["name"], json!("ok"));
    }

    #[test]
    fn reports_not_redacted_when_secret_fields_are_already_blank() {
        let mut v = json!({ "type": "postgres", "uri": "", "table": "t" });
        assert!(!redact_secrets(&mut v));
    }

    #[test]
    fn does_not_match_substring_keys() {
        // `secret_count` is not the secret `secret`; leave it alone.
        let mut v = json!({ "secret_count": 3 });
        assert!(!redact_secrets(&mut v));
        assert_eq!(v["secret_count"], json!(3));
    }
}
