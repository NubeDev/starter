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

/// Run the load-time lints over `kind` — the same checks the boot loader applies
/// to file-pack kinds, exposed so the API layer can reject an unsafe
/// tenant-authored kind (§4.5c) *before* it is persisted: an undeclared `$param`,
/// a host token smuggled as a param, or a missing `$caller_tenant_id` predicate
/// on a tenant-scoped table fails the save with a 4xx instead of writing a row
/// that would later fail (or leak) at dispatch. A persisted kind is therefore
/// always already lint-clean, exactly like a file kind.
pub fn lint(kind: &QueryKind) -> Result<(), KindError> {
    lint::check(kind)
}

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

    /// Build a registry from an in-memory set of kinds, refusing duplicate
    /// names. The source-agnostic counterpart to [`Self::load_dir`]: WS-14 uses
    /// it to assemble the **extension-contributed** kinds (the third source)
    /// from `nexus_extension_query_kinds` rows at boot, so the dispatcher
    /// resolves them through the identical validate/bind path as file kinds with
    /// no per-request DB hit. A duplicate name is an error — the global
    /// `UNIQUE (name)` on the table makes that unreachable in practice, but the
    /// registry stays the single place that invariant is enforced in memory.
    pub fn from_kinds(
        kinds: impl IntoIterator<Item = QueryKind>,
    ) -> Result<Self, KindError> {
        let mut by_name = BTreeMap::new();
        for kind in kinds {
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

/// Validate caller `params` for the named kind in the file registry and produce
/// the kind's SQL plus the binder param map. A request-time entry point: the
/// dispatcher then hands the result to the store, which binds it (params + host
/// tokens) and runs it. A name absent from the registry is `Unknown` — the
/// dispatcher treats that as a cue to look for a tenant-authored kind (§4.5c).
pub fn resolve(
    registry: &Registry,
    name: &str,
    params: &serde_json::Value,
) -> Result<BoundKind, KindError> {
    let kind = registry
        .get(name)
        .ok_or_else(|| KindError::Unknown(name.to_string()))?;
    resolve_kind(kind, params)
}

/// Validate caller `params` against an already-resolved [`QueryKind`] and lower
/// them to the binder param map. The source of the kind — file pack or the
/// metadata DB — is irrelevant here: a tenant-authored kind is reconstructed into
/// a `QueryKind` and validated through this exact same path, so both honour the
/// identical schema/host-token rules.
pub fn resolve_kind(kind: &QueryKind, params: &serde_json::Value) -> Result<BoundKind, KindError> {
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

    /// A tenant-authored kind (§4.5c) is reconstructed into a `QueryKind` from a DB
    /// row and resolved through `resolve_kind` — the same path file kinds use. This
    /// proves the keystone property: a DB-sourced kind honours the identical
    /// host-token rule, so a caller still cannot smuggle `$caller_tenant_id`.
    fn db_kind(sql: &str, schema: serde_json::Value) -> QueryKind {
        QueryKind {
            name: "com.acme.saved".into(),
            sql: sql.into(),
            params_schema: schema,
            datasource_kind: "postgres".into(),
            tables: vec!["meters".into()],
            datasource_binding: None,
            description: None,
        }
    }

    #[test]
    fn resolve_kind_on_a_db_kind_rejects_smuggled_host_token() {
        let kind = db_kind(
            "SELECT 1 FROM meters WHERE tenant_id = $caller_tenant_id",
            serde_json::json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        );
        let err = resolve_kind(&kind, &serde_json::json!({ "caller_tenant_id": "other" }))
            .expect_err("a DB kind must reject a smuggled host token like a file kind");
        assert!(matches!(err, KindError::ParamValidation { .. }));
    }

    #[test]
    fn lint_rejects_a_db_kind_missing_the_tenant_predicate() {
        // The save handler calls `lint` before persisting; a kind that reads a
        // tenant-scoped table but omits `$caller_tenant_id` must fail, so an unsafe
        // row never reaches the DB (the data side has no RLS — §4.4).
        let unsafe_kind = db_kind(
            "SELECT 1 FROM meters",
            serde_json::json!({ "type": "object", "properties": {} }),
        );
        let err = lint(&unsafe_kind).expect_err("a missing tenant predicate must fail the lint");
        assert!(matches!(err, KindError::Lint { .. }));

        // The same kind with the predicate present lints clean.
        let safe_kind = db_kind(
            "SELECT 1 FROM meters WHERE tenant_id = $caller_tenant_id",
            serde_json::json!({ "type": "object", "properties": {} }),
        );
        assert!(lint(&safe_kind).is_ok());
    }
}
