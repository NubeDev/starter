//! Declarative query-kinds: named, schema-validated, parameterized queries
//! declared as files instead of pasted raw SQL.
//!
//! A *kind* is a unit of API surface declared by manifest + convention (ported
//! from the rubix `kinds/` pattern). A panel invokes a kind by reverse-DNS name
//! with validated params instead of sending arbitrary SQL: the [`Registry`]
//! resolves the name, validates params against the kind's JSON Schema, and binds
//! them — plus the host-bound `$caller_tenant_id` — through the shared C2 binder
//! (`nexus_store::bind`). The kind's SQL still runs under the existing read-only/
//! timeout/cap guards; kinds are the *safe declarative front door*, not a second
//! engine. See docs/design/query/ for the binder; the rubix origin is the
//! `com.nubeio.rubixos` extension.

mod dispatch;
mod error;
mod kind;
mod lint;
mod load;
mod manifest;
mod resolve;
mod validate;

use std::collections::BTreeMap;
use std::path::Path;

pub use dispatch::run;
pub use error::KindError;
pub use kind::QueryKind;
pub use resolve::BoundKind;

/// An immutable, source-agnostic set of registered query-kinds keyed by name.
///
/// v1 loads a built-in pack directory at boot; the registry itself is unaware of
/// *where* kinds came from, so extension- or DB-sourced kinds (§4.5b/c) plug in
/// later without re-architecting. Cloning is cheap-ish (the kinds are small) and
/// the registry is shared read-only across requests via [`AppState`].
#[derive(Debug, Clone, Default)]
pub struct Registry {
    by_name: BTreeMap<String, QueryKind>,
}

impl Registry {
    /// An empty registry — the deployment-has-no-kinds default.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Load every kind from the pack directory at `dir`, refusing duplicate
    /// names. A missing directory yields an empty registry; a malformed pack is
    /// an error so a boot-time typo aborts startup loudly.
    pub fn load_dir(dir: &Path) -> Result<Self, KindError> {
        let mut by_name = BTreeMap::new();
        for kind in load::load_pack(dir)? {
            if by_name.contains_key(&kind.name) {
                return Err(KindError::DuplicateName(kind.name));
            }
            by_name.insert(kind.name.clone(), kind);
        }
        Ok(Self { by_name })
    }

    /// Look up a registered kind by its reverse-DNS name.
    pub fn get(&self, name: &str) -> Option<&QueryKind> {
        self.by_name.get(name)
    }

    /// The kinds in the registry, name-ordered — for the picker endpoint/UI.
    pub fn iter(&self) -> impl Iterator<Item = &QueryKind> {
        self.by_name.values()
    }

    /// How many kinds are registered.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry holds no kinds.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// Validate caller `params` for the named kind and produce the kind's SQL plus
/// the binder param map. A request-time entry point: the dispatcher then hands
/// the result to the store, which binds it (params + host tokens) and runs it.
pub fn resolve(
    registry: &Registry,
    name: &str,
    params: &serde_json::Value,
) -> Result<BoundKind, KindError> {
    let kind = registry
        .get(name)
        .ok_or_else(|| KindError::Unknown(name.to_string()))?;
    let params = validate::validate(kind, params)?;
    Ok(BoundKind {
        sql: kind.sql.clone(),
        params,
        datasource_kind: kind.datasource_kind.clone(),
        datasource_binding: kind.datasource_binding.clone(),
        tables: kind.tables.clone(),
    })
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
                "nexus-kinds-test-{label}-{}-{:?}",
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

    /// The built-in core pack shipped beside this crate.
    fn builtin_pack_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("kinds")
    }

    #[test]
    fn missing_directory_loads_an_empty_registry() {
        let dir = std::env::temp_dir().join("nexus-kinds-does-not-exist-xyz");
        let reg = Registry::load_dir(&dir).expect("a missing pack dir is not an error");
        assert!(reg.is_empty());
    }

    #[test]
    fn builtin_core_pack_loads_and_lints_clean() {
        let reg = Registry::load_dir(&builtin_pack_dir()).expect("the shipped pack must load");
        assert_eq!(reg.len(), 4, "the core pack declares four kinds");
        assert!(reg.get("nexus.core.meters_list").is_some());
        assert!(reg.get("nexus.core.meter_get").is_some());
        assert!(reg.get("nexus.core.usage_bucketed").is_some());
        assert!(reg.get("nexus.core.top_sites_by_usage").is_some());
    }

    #[test]
    fn duplicate_kind_name_is_rejected() {
        let pack = ScratchPack::new("dup");
        pack.write("q.sql", "SELECT 1");
        pack.write("p.json", r#"{"type":"object","properties":{}}"#);
        pack.write(
            "manifest.yaml",
            "query_kinds:\n\
             \x20 - name: nexus.test.dup\n\
             \x20   params_schema: p.json\n\
             \x20   sql_file: q.sql\n\
             \x20   datasource_kind: postgres\n\
             \x20   tables: []\n\
             \x20 - name: nexus.test.dup\n\
             \x20   params_schema: p.json\n\
             \x20   sql_file: q.sql\n\
             \x20   datasource_kind: postgres\n\
             \x20   tables: []\n",
        );
        let err = Registry::load_dir(&pack.dir).expect_err("two kinds with one name must fail");
        assert!(matches!(err, KindError::DuplicateName(_)));
    }

    #[test]
    fn missing_sql_file_fails_the_load() {
        let pack = ScratchPack::new("missing-sql");
        pack.write("p.json", r#"{"type":"object","properties":{}}"#);
        pack.write(
            "manifest.yaml",
            "query_kinds:\n\
             \x20 - name: nexus.test.nofile\n\
             \x20   params_schema: p.json\n\
             \x20   sql_file: absent.sql\n\
             \x20   datasource_kind: postgres\n\
             \x20   tables: []\n",
        );
        let err = Registry::load_dir(&pack.dir).expect_err("a referenced-but-absent file fails");
        assert!(matches!(err, KindError::KindFile { .. }));
    }

    #[test]
    fn resolve_rejects_unknown_kind() {
        let reg = Registry::load_dir(&builtin_pack_dir()).unwrap();
        let err = resolve(&reg, "nexus.core.nope", &serde_json::json!({}))
            .expect_err("an unregistered name is an error");
        assert!(matches!(err, KindError::Unknown(_)));
    }

    #[test]
    fn resolve_validates_params_and_binds_host_tenant_predicate() {
        let reg = Registry::load_dir(&builtin_pack_dir()).unwrap();
        let bound = resolve(&reg, "nexus.core.meters_list", &serde_json::json!({ "site_id": "s1" }))
            .expect("valid params resolve");
        // The kind's SQL keeps the host-bound tenant predicate; the caller cannot
        // supply $caller_tenant_id as a param (it is not a declared property).
        assert!(bound.sql.contains("$caller_tenant_id"));
        assert_eq!(bound.datasource_kind, "postgres");
        assert!(bound.params.contains_key("site_id"));
        assert!(
            !bound.params.contains_key("caller_tenant_id"),
            "the host token is never a caller-supplied param"
        );
    }

    #[test]
    fn resolve_rejects_attempt_to_pass_host_token_as_param() {
        let reg = Registry::load_dir(&builtin_pack_dir()).unwrap();
        // meters_list's schema is additionalProperties:false, so smuggling a
        // caller_tenant_id key is rejected at validation, never reaching the binder.
        let err = resolve(
            &reg,
            "nexus.core.meters_list",
            &serde_json::json!({ "caller_tenant_id": "other-tenant" }),
        )
        .expect_err("a host token is not a declarable param");
        assert!(matches!(err, KindError::ParamValidation { .. }));
    }
}
