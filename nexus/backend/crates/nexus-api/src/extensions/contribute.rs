//! Read an extension's `contributes.warehouse_templates[]` into nexus
//! query-kinds.
//!
//! WS-14 §5 / §9 Q1: an extension's `warehouse_templates[]` *are* query-kinds —
//! the cleanest backend contribution. A template declares a `name`, a
//! bundle-relative `params_schema` JSON file, the `tables` it reads, and a
//! bundle-relative `sql_file`. This module reads those files off disk
//! (`ExtensionRecord::bundle_dir`), maps each into a [`NewExtensionQueryKind`],
//! and **lints it through the exact same gate file-pack and tenant kinds pass**
//! ([`crate::kinds::lint`]) — so an extension can never contribute a kind that
//! reads a tenant-scoped table without a `$caller_tenant_id` guard, or
//! references an undeclared `$param`. A template that fails the lint is rejected
//! (the extension's contribution is refused), never silently materialised.
//!
//! The result feeds two callers: the boot loader (materialise every enabled
//! extension's templates into `nexus_extension_query_kinds`, then build the
//! in-memory registry) and the install post-hook (materialise a freshly-
//! uploaded bundle's templates immediately).

use std::path::Path;

use starter_ext_spi::Manifest;

use crate::kinds::QueryKind;
use nexus_store::extension_query_kind::NewExtensionQueryKind;

/// Why a contributed template could not be turned into a query-kind.
#[derive(Debug, thiserror::Error)]
pub enum ContributeError {
    /// A bundle-relative file (`params_schema` or `sql_file`) was missing or
    /// unreadable.
    #[error("extension `{extension}` template `{template}`: reading {path}: {source}")]
    File {
        extension: String,
        template: String,
        path: String,
        source: std::io::Error,
    },
    /// The `params_schema` file held invalid JSON.
    #[error(
        "extension `{extension}` template `{template}`: params_schema is not valid JSON: {source}"
    )]
    Schema {
        extension: String,
        template: String,
        source: serde_json::Error,
    },
    /// A template declared no `sql_file`. nexus query-kinds are SQL templates,
    /// so a template contributing a kind must carry one (a warehouse template
    /// with no SQL body is meaningful to rubix's warehouse engine but not to
    /// the nexus query path — those are simply skipped, not an error).
    #[error("extension `{extension}` template `{template}`: no sql_file — cannot contribute a query-kind")]
    NoSql { extension: String, template: String },
    /// The reconstructed kind failed the load-time lint (undeclared `$param`,
    /// missing `$caller_tenant_id` predicate on a tenant-scoped table, or a
    /// smuggled host token). The same gate file-pack kinds pass.
    #[error("extension `{extension}` template `{template}`: lint rejected the contributed kind: {source}")]
    Lint {
        extension: String,
        template: String,
        source: crate::kinds::KindError,
    },
}

/// Read every query-kind an extension contributes via its
/// `warehouse_templates[]`, lint-validating each. `bundle_dir` is the
/// extension's on-disk root (`ExtensionRecord::bundle_dir`); `extension_id` is
/// the owner recorded on each row.
///
/// Templates without a `sql_file` are **skipped** (they target rubix's
/// warehouse DDL path, not the nexus query path) rather than erroring, so a
/// bundle that mixes warehouse-table templates and query-kind templates
/// contributes only the latter here. Every template that *does* carry SQL must
/// lint clean or the whole call errors — a partial, half-validated contribution
/// is never returned.
pub fn contributed_query_kinds(
    extension_id: &str,
    bundle_dir: &Path,
    manifest: &Manifest,
) -> Result<Vec<NewExtensionQueryKind>, ContributeError> {
    let mut out = Vec::new();
    for tpl in &manifest.contributes.warehouse_templates {
        // A template with no SQL body is a warehouse-DDL template (rubix's
        // path), not a query-kind. Skip it here.
        let Some(sql_rel) = tpl.sql_file.as_deref() else {
            continue;
        };

        let sql = read_bundle_file(extension_id, &tpl.name, bundle_dir, sql_rel)?;
        let schema_raw = read_bundle_file(extension_id, &tpl.name, bundle_dir, &tpl.params_schema)?;
        let params_schema: serde_json::Value =
            serde_json::from_str(&schema_raw).map_err(|source| ContributeError::Schema {
                extension: extension_id.to_string(),
                template: tpl.name.clone(),
                source,
            })?;

        let kind = QueryKind {
            name: tpl.name.clone(),
            sql,
            params_schema: params_schema.clone(),
            // Templates declare the tables they touch; the datasource shape they
            // target is `postgres` for the nexus metadata query path (the only
            // shape kinds bind against today). A future manifest field could
            // carry this explicitly; until then postgres is the contract.
            datasource_kind: "postgres".to_string(),
            tables: tpl.tables.clone(),
            datasource_binding: None,
            description: None,
        };

        // The keystone safety check: an extension-contributed kind passes the
        // identical lint as a file-pack or tenant kind. A tenant-scoped table
        // without `$caller_tenant_id`, or an undeclared `$param`, is rejected
        // here — the contribution is refused, never persisted.
        crate::kinds::lint(&kind).map_err(|source| ContributeError::Lint {
            extension: extension_id.to_string(),
            template: tpl.name.clone(),
            source,
        })?;

        out.push(NewExtensionQueryKind {
            name: kind.name,
            sql: kind.sql,
            params_schema: kind.params_schema,
            datasource_kind: kind.datasource_kind,
            tables: kind.tables,
            datasource_binding: kind.datasource_binding,
            description: kind.description,
        });
    }
    Ok(out)
}

/// Read a bundle-relative file, mapping IO errors to [`ContributeError::File`]
/// with the template context attached.
fn read_bundle_file(
    extension_id: &str,
    template: &str,
    bundle_dir: &Path,
    rel: &str,
) -> Result<String, ContributeError> {
    let path = bundle_dir.join(rel);
    std::fs::read_to_string(&path).map_err(|source| ContributeError::File {
        extension: extension_id.to_string(),
        template: template.to_string(),
        path: path.display().to_string(),
        source,
    })
}

/// Map a [`NewExtensionQueryKind`] back into a [`QueryKind`] for registry
/// assembly. The boot loader persists the `New*` rows, reads them back as
/// records, and builds the in-memory registry from this projection so the
/// dispatcher resolves them through the standard path.
pub fn record_to_query_kind(
    rec: nexus_store::extension_query_kind::ExtensionQueryKindRecord,
) -> QueryKind {
    QueryKind {
        name: rec.name,
        sql: rec.sql,
        params_schema: rec.params_schema,
        datasource_kind: rec.datasource_kind,
        tables: rec.tables,
        datasource_binding: rec.datasource_binding,
        description: rec.description,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// The in-repo example extension shipped beside this crate.
    fn example_bundle() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("extensions/com.nexus.hello")
    }

    /// The whole boot-side contribution path against the real shipped bundle:
    /// the manifest validates through the kernel loader, and both
    /// `warehouse_templates[]` read + lint into query-kinds. This is the same
    /// path `load_extension_kinds` runs at boot, minus the DB upsert.
    #[test]
    fn example_bundle_contributes_lint_clean_kinds() {
        let mut registry = starter_ext_host::ExtensionRegistry::new();
        let records =
            starter_ext_host::Loader::scan(example_bundle().parent().unwrap()).validate_all();
        let outcome = starter_ext_host::Loader::commit(records, &mut registry);
        assert_eq!(
            outcome.failed, 0,
            "the shipped example bundle must validate"
        );
        registry.seal();

        let record = registry
            .get_by_id_str("com.nexus.hello")
            .expect("com.nexus.hello is in the pack");
        let manifest = record.manifest.as_ref().expect("validated ⇒ manifest");

        let kinds = contributed_query_kinds("com.nexus.hello", &record.bundle_dir, manifest)
            .expect("shipped templates must read + lint clean");
        assert_eq!(kinds.len(), 2, "ping + echo");
        assert!(kinds.iter().any(|k| k.name == "com.nexus.hello.ping"));
        assert!(kinds.iter().any(|k| k.name == "com.nexus.hello.echo"));

        // And they assemble into a registry the dispatcher can resolve.
        let reg = crate::kinds::Registry::from_kinds(kinds.iter().map(|k| QueryKind {
            name: k.name.clone(),
            sql: k.sql.clone(),
            params_schema: k.params_schema.clone(),
            datasource_kind: k.datasource_kind.clone(),
            tables: k.tables.clone(),
            datasource_binding: k.datasource_binding.clone(),
            description: k.description.clone(),
        }))
        .expect("no duplicate names");
        let bound = crate::kinds::resolve(
            &reg,
            "com.nexus.hello.echo",
            &serde_json::json!({ "message": "works" }),
        )
        .expect("declared param validates");
        assert!(bound.sql.contains("$message"));
        assert!(bound.params.contains_key("message"));
    }
}
