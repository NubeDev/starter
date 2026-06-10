//! The YAML manifest that declares a kinds pack — the nexus-native port of the
//! rubix `block.yaml` `warehouse_templates:` shape.
//!
//! A manifest is data only: it names each query-kind and points at its SQL and
//! params-schema files by relative path. The loader (`load.rs`) resolves those
//! paths, reads the files, runs the lints, and builds the in-memory registry.
//! Field names are kept aligned to the rubix mental model so the pattern
//! transfers, but the format is nexus's own (`query_kinds:`, not
//! `warehouse_templates:`).

use serde::Deserialize;

/// A parsed pack manifest: the list of query-kinds it contributes.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    /// The query-kinds this pack declares. Each entry is a file triple wired by
    /// the manifest, not by code.
    #[serde(default)]
    pub query_kinds: Vec<ManifestEntry>,
}

/// One query-kind declaration. `name` is the reverse-DNS id a caller invokes;
/// the file paths are relative to the manifest's directory.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestEntry {
    /// Reverse-DNS kind id (e.g. `nexus.energy.usage_bucketed`). A request names
    /// this; the registry refuses anything it does not hold.
    pub name: String,

    /// Relative path to the JSON-Schema params file
    /// (`additionalProperties: false`, defaults, min/max). Validated before any
    /// SQL runs.
    pub params_schema: String,

    /// Relative path to the SQL file. The SQL uses `$caller_tenant_id` (host
    /// bound), `$__time*` macros, and `$param` references the schema declares.
    pub sql_file: String,

    /// Which datasource shape this kind targets (e.g. `postgres`). Resolved
    /// against the bound datasource's `kind` at dispatch; the panel still
    /// carries its own `datasourceId`.
    pub datasource_kind: String,

    /// Tables the kind reads. Drives the read-capability surface and (later, with
    /// WS-09) cache invalidation. Used by the lint to know which references are
    /// tenant-scoped tables that must carry the `$caller_tenant_id` predicate.
    #[serde(default)]
    pub tables: Vec<String>,

    /// Optional pinned datasource id. When set, the kind only runs against that
    /// specific datasource (a curated core pack); when unset, any datasource of
    /// `datasource_kind` the caller can view is valid.
    #[serde(default)]
    pub datasource_binding: Option<String>,

    /// Optional human description for the kind picker UI.
    #[serde(default)]
    pub description: Option<String>,
}
