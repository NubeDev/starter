//! Declarative datasource-kinds: connectors declared as files (a config schema,
//! its secret fields, a connectivity-test descriptor, and an optional SQL
//! dialect) instead of a Rust enum edited across DTOs, forms, and the registry.
//!
//! This is the datasource half of the rubix `kinds/` pattern (WS-10 §4.1B), a
//! sibling of the query-kind registry (`crate::kinds`). A datasource-kind names a
//! connector *type* (`postgres`, `mqtt`): its [`DatasourceKind::config_schema`]
//! validates a config before save, its `secret_fields` drive the seal/redact/
//! decrypt boundary, and its `test` descriptor selects the connect/probe path.
//! Adding a connector becomes a manifest entry plus a thin per-protocol builder,
//! not enum edits scattered across the tree. The query-kind binder is untouched —
//! this registry is about *where data comes from*, not the queries run against it.
//! See docs/design/datasources/ for the connector boundary.

mod error;
mod kind;
mod lint;
mod load;
mod manifest;
mod resolve_output;
mod validate;

use std::collections::BTreeMap;
use std::path::Path;

pub use error::DatasourceKindError;
pub use kind::DatasourceKind;
pub use manifest::{Surface, TestSpec};
pub use resolve_output::resolve_flow_output;

/// An immutable, source-agnostic set of registered datasource-kinds keyed by
/// name.
///
/// v1 loads a built-in pack directory at boot; the registry itself is unaware of
/// *where* kinds came from, so extension- or DB-sourced connectors plug in later
/// without re-architecting. The registry is shared read-only across requests via
/// [`AppState`].
#[derive(Debug, Clone, Default)]
pub struct Registry {
    by_name: BTreeMap<String, DatasourceKind>,
}

impl Registry {
    /// An empty registry — the deployment-has-no-declared-connectors default.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Load every datasource-kind from the pack directory at `dir`, refusing
    /// duplicate names. A missing directory yields an empty registry; a malformed
    /// pack is an error so a boot-time typo aborts startup loudly.
    pub fn load_dir(dir: &Path) -> Result<Self, DatasourceKindError> {
        let mut by_name = BTreeMap::new();
        for kind in load::load_pack(dir)? {
            if by_name.contains_key(&kind.name) {
                return Err(DatasourceKindError::DuplicateName(kind.name));
            }
            by_name.insert(kind.name.clone(), kind);
        }
        Ok(Self { by_name })
    }

    /// Look up a registered datasource-kind by its id.
    pub fn get(&self, name: &str) -> Option<&DatasourceKind> {
        self.by_name.get(name)
    }

    /// The datasource-kinds in the registry, name-ordered — for the catalogue
    /// endpoint that drives per-kind config forms.
    pub fn iter(&self) -> impl Iterator<Item = &DatasourceKind> {
        self.by_name.values()
    }

    /// How many datasource-kinds are registered.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry holds no datasource-kinds.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// Validate a connector `config` for the named datasource-kind, returning the
/// merged config (defaults applied) ready to seal + persist. A request-time entry
/// point used by the create and test paths.
pub fn validate_config(
    registry: &Registry,
    name: &str,
    config: &serde_json::Value,
) -> Result<serde_json::Value, DatasourceKindError> {
    let kind = registry
        .get(name)
        .ok_or_else(|| DatasourceKindError::Unknown(name.to_string()))?;
    validate::validate(kind, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A scratch pack directory under the temp dir, removed on drop so a failed
    /// test never leaks a fixture or pollutes the next run.
    struct ScratchPack {
        dir: PathBuf,
    }

    impl ScratchPack {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "nexus-dskinds-test-{label}-{}-{:?}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self { dir }
        }

        fn write(&self, name: &str, contents: &str) {
            std::fs::write(self.dir.join(name), contents).unwrap();
        }
    }

    impl Drop for ScratchPack {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// The built-in datasource-kinds pack shipped beside this crate.
    fn builtin_pack_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datasource-kinds")
    }

    #[test]
    fn missing_directory_loads_an_empty_registry() {
        let dir = std::env::temp_dir().join("nexus-dskinds-does-not-exist-xyz");
        let reg = Registry::load_dir(&dir).expect("a missing pack dir is not an error");
        assert!(reg.is_empty());
    }

    #[test]
    fn builtin_pack_loads_and_lints_clean() {
        let reg = Registry::load_dir(&builtin_pack_dir()).expect("the shipped pack must load");
        assert_eq!(reg.len(), 2, "the core pack declares postgres + mqtt");
        let pg = reg.get("postgres").expect("postgres is declared");
        assert_eq!(pg.surface, Surface::Query);
        assert!(pg.secret_fields.contains(&"password".to_string()));
        let mqtt = reg.get("mqtt").expect("mqtt is declared");
        assert_eq!(mqtt.surface, Surface::Stream);
    }

    #[test]
    fn duplicate_kind_name_is_rejected() {
        let pack = ScratchPack::new("dup");
        pack.write("c.json", r#"{"type":"object","properties":{}}"#);
        pack.write(
            "manifest.yaml",
            "datasource_kinds:\n\
             \x20 - name: dup\n\
             \x20   surface: stream\n\
             \x20   config_schema: c.json\n\
             \x20   test:\n\
             \x20     mode: connect\n\
             \x20 - name: dup\n\
             \x20   surface: stream\n\
             \x20   config_schema: c.json\n\
             \x20   test:\n\
             \x20     mode: connect\n",
        );
        let err = Registry::load_dir(&pack.dir).expect_err("two kinds with one name must fail");
        assert!(matches!(err, DatasourceKindError::DuplicateName(_)));
    }

    #[test]
    fn missing_config_schema_file_fails_the_load() {
        let pack = ScratchPack::new("missing-schema");
        pack.write(
            "manifest.yaml",
            "datasource_kinds:\n\
             \x20 - name: nofile\n\
             \x20   surface: stream\n\
             \x20   config_schema: absent.json\n\
             \x20   test:\n\
             \x20     mode: connect\n",
        );
        let err = Registry::load_dir(&pack.dir).expect_err("a referenced-but-absent file fails");
        assert!(matches!(err, DatasourceKindError::KindFile { .. }));
    }

    #[test]
    fn validate_config_rejects_unknown_kind() {
        let reg = Registry::load_dir(&builtin_pack_dir()).unwrap();
        let err = validate_config(&reg, "nope", &serde_json::json!({}))
            .expect_err("an unregistered name is an error");
        assert!(matches!(err, DatasourceKindError::Unknown(_)));
    }

    #[test]
    fn validate_config_applies_mqtt_defaults_and_rejects_bad_qos() {
        let reg = Registry::load_dir(&builtin_pack_dir()).unwrap();
        let cfg = validate_config(
            &reg,
            "mqtt",
            &serde_json::json!({ "host": "broker.example", "topic": "sensors/#" }),
        )
        .expect("a minimal mqtt config validates with defaults");
        // The schema's defaults fill the omitted port + qos.
        assert_eq!(cfg.get("port"), Some(&serde_json::json!(1883)));
        let err = validate_config(
            &reg,
            "mqtt",
            &serde_json::json!({ "host": "b", "topic": "t", "qos": 5 }),
        )
        .expect_err("qos is bounded 0..=2");
        assert!(matches!(err, DatasourceKindError::ConfigValidation { .. }));
    }
}
