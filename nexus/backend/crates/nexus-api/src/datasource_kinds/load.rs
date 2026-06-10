//! Load a datasource-kinds pack directory into a list of validated
//! [`DatasourceKind`]s.
//!
//! A pack is a directory holding a `manifest.yaml` plus the `*_config.json`
//! schema files it references by relative path. Loading reads the manifest,
//! resolves each entry's config-schema file, parses it, builds the
//! `DatasourceKind`, and runs the boot-time lints (`lint::check`). Any failure
//! aborts the load — a malformed pack must never reach the registry.

use std::path::Path;

use serde_json::Value;

use super::error::DatasourceKindError;
use super::kind::DatasourceKind;
use super::lint;
use super::manifest::Manifest;

/// The manifest filename a pack directory is expected to contain.
pub const MANIFEST_FILE: &str = "manifest.yaml";

/// Read and validate every datasource-kind declared by the pack at `dir`. The
/// returned kinds are lint-clean and ready to register. A missing directory
/// yields an empty list, but a present-but-malformed pack is an error so a typo
/// is loud, not silent.
pub fn load_pack(dir: &Path) -> Result<Vec<DatasourceKind>, DatasourceKindError> {
    let manifest_path = dir.join(MANIFEST_FILE);
    if !manifest_path.exists() {
        return Ok(Vec::new());
    }
    let text =
        std::fs::read_to_string(&manifest_path).map_err(|e| DatasourceKindError::Manifest {
            path: manifest_path.display().to_string(),
            source: Box::new(e),
        })?;
    let manifest: Manifest =
        serde_yaml::from_str(&text).map_err(|e| DatasourceKindError::Manifest {
            path: manifest_path.display().to_string(),
            source: Box::new(e),
        })?;

    let mut kinds = Vec::with_capacity(manifest.datasource_kinds.len());
    for entry in manifest.datasource_kinds {
        let kind = build_kind(dir, &entry)?;
        lint::check(&kind)?;
        kinds.push(kind);
    }
    Ok(kinds)
}

/// Build one [`DatasourceKind`] by reading its config-schema file relative to the
/// pack directory and parsing it.
fn build_kind(
    dir: &Path,
    entry: &super::manifest::ManifestEntry,
) -> Result<DatasourceKind, DatasourceKindError> {
    let schema_text = read_file(dir, &entry.config_schema, &entry.name)?;
    let config_schema: Value =
        serde_json::from_str(&schema_text).map_err(|e| DatasourceKindError::SchemaParse {
            kind: entry.name.clone(),
            source: e,
        })?;
    Ok(DatasourceKind {
        name: entry.name.clone(),
        surface: entry.surface,
        config_schema,
        secret_fields: entry.secret_fields.clone(),
        test: entry.test.clone(),
        dialect: entry.dialect.clone(),
        description: entry.description.clone(),
    })
}

/// Read a pack-relative file, mapping a missing/unreadable file to a kind-scoped
/// error so the failure names the offending kind.
fn read_file(dir: &Path, relative: &str, kind: &str) -> Result<String, DatasourceKindError> {
    let path = dir.join(relative);
    std::fs::read_to_string(&path).map_err(|e| DatasourceKindError::KindFile {
        kind: kind.to_string(),
        path: path.display().to_string(),
        source: e,
    })
}
