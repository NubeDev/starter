//! Load a kinds pack directory into a list of validated [`QueryKind`]s.
//!
//! A pack is a directory holding a `manifest.yaml` plus the `*.sql` and
//! `*_params.json` files it references by relative path. Loading reads the
//! manifest, resolves each entry's files, parses the params schema, builds the
//! `QueryKind`, and runs the boot-time lints (`lint::check`). Any failure aborts
//! the load — a malformed pack must never reach the registry.

use std::path::Path;

use serde_json::Value;

use super::error::KindError;
use super::kind::QueryKind;
use super::lint;
use super::manifest::Manifest;

/// The manifest filename a pack directory is expected to contain.
pub const MANIFEST_FILE: &str = "manifest.yaml";

/// Read and validate every kind declared by the pack at `dir`. The returned
/// kinds are lint-clean and ready to register. A missing directory yields an
/// empty list (a deployment may ship no kinds), but a present-but-malformed pack
/// is an error so a typo is loud, not silent.
pub fn load_pack(dir: &Path) -> Result<Vec<QueryKind>, KindError> {
    let manifest_path = dir.join(MANIFEST_FILE);
    if !manifest_path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&manifest_path).map_err(|e| KindError::Manifest {
        path: manifest_path.display().to_string(),
        source: Box::new(e),
    })?;
    let manifest: Manifest = serde_yaml::from_str(&text).map_err(|e| KindError::Manifest {
        path: manifest_path.display().to_string(),
        source: Box::new(e),
    })?;

    let mut kinds = Vec::with_capacity(manifest.query_kinds.len());
    for entry in manifest.query_kinds {
        let kind = build_kind(dir, &entry)?;
        lint::check(&kind)?;
        kinds.push(kind);
    }
    Ok(kinds)
}

/// Build one [`QueryKind`] by reading its SQL and params-schema files relative to
/// the pack directory and parsing the schema.
fn build_kind(dir: &Path, entry: &super::manifest::ManifestEntry) -> Result<QueryKind, KindError> {
    let sql = read_file(dir, &entry.sql_file, &entry.name)?;
    let schema_text = read_file(dir, &entry.params_schema, &entry.name)?;
    let params_schema: Value =
        serde_json::from_str(&schema_text).map_err(|e| KindError::SchemaParse {
            kind: entry.name.clone(),
            source: e,
        })?;
    Ok(QueryKind {
        name: entry.name.clone(),
        sql,
        params_schema,
        datasource_kind: entry.datasource_kind.clone(),
        tables: entry.tables.clone(),
        datasource_binding: entry.datasource_binding.clone(),
        description: entry.description.clone(),
    })
}

/// Read a pack-relative file, mapping a missing/unreadable file to a kind-scoped
/// error so the failure names the offending kind.
fn read_file(dir: &Path, relative: &str, kind: &str) -> Result<String, KindError> {
    let path = dir.join(relative);
    std::fs::read_to_string(&path).map_err(|e| KindError::KindFile {
        kind: kind.to_string(),
        path: path.display().to_string(),
        source: e,
    })
}
