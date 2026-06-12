//! In-process registry of warehouse-read templates.
//!
//! Per [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
//! row 2 and Appendix A of
//! [`docs/proposal/extension-architecture-north-star.md`](../../../../rubix/docs/proposal/extension-architecture-north-star.md):
//! the `WarehouseReadHandle` is **not** a SQL gateway. The host
//! registers a finite catalog of named templates; extensions
//! reference them by name and bind typed parameters. This module
//! is the catalog.
//!
//! The registry stores [`TemplateSpec`] metadata (re-exported from
//! `starter-ext-spi::warehouse`). Concrete resolvers — the code that
//! binds parameters and runs SQL — live in the host integration
//! crate (`rubix-agent` for the rubix product) and call into the
//! registry only for spec lookup. Keeping resolvers out of this
//! crate matches the layering rule that `starter-ext-host` is
//! I/O-free outside manifest loading: it does not depend on
//! `sqlx`, on a warehouse client, or on any tenant-store crate.
//!
//! ## Builtin templates
//!
//! [`TemplateRegistry::builtin`] returns the four templates currently
//! hard-coded inside
//! [`rubix/crates/rubix-agent/src/sdui/analytics_bridge.rs`](../../../../rubix/crates/rubix-agent/src/sdui/analytics_bridge.rs):
//!
//! | name                    | tables   | params                  |
//! |-------------------------|----------|-------------------------|
//! | `meter_kwh_last_24h`    | samples  | tenant_id               |
//! | `meter_litres_last_24h` | samples  | tenant_id               |
//! | `meter_value_30d_15m`   | samples  | tenant_id, meter_id     |
//! | `meter_value_24h_1m`    | samples  | tenant_id, meter_id     |
//!
//! The SQL body is captured in [`TemplateSpec::sql`] for audit /
//! documentation surfaces. The host's resolver matches by name; the
//! string is not executed by this module.
//!
//! ## Extension-contributed templates
//!
//! Row 3 of the critical path adds the
//! `contributes.warehouse_templates[]` manifest slice. When that
//! lands, `Loader::commit` will call [`TemplateRegistry::with`] for
//! each contributed spec so the registry is a single audit surface
//! across builtin + contributed entries.

use std::collections::BTreeMap;
use std::path::Path;

use starter_ext_spi::manifest::ContributeWarehouseTemplate;
use starter_ext_spi::warehouse::TemplateSpec;
use starter_ext_spi::{Error, ExtensionId};
use tracing::warn;

use crate::record::ExtensionRecord;

/// In-process catalog of named warehouse-read templates.
///
/// Construction is one of:
/// - `TemplateRegistry::builtin()` — populated with the four host
///   templates `AnalyticsBridge` already resolves.
/// - `TemplateRegistry::empty()` — bare registry for tests.
///
/// Insertion is via [`TemplateRegistry::with`] (consuming, builder
/// style) for static composition, or [`TemplateRegistry::insert`]
/// for mutable composition during boot. After boot the registry
/// is read-only by convention — callers wrap it in an `Arc`.
#[derive(Debug, Default, Clone)]
pub struct TemplateRegistry {
    by_name: BTreeMap<String, TemplateSpec>,
    /// Side map keyed identically to `by_name`. Present entries name
    /// the extension that contributed the template; absent entries
    /// mean "host-trusted builtin, no per-extension allowlist
    /// applies". The bridge gate keys off this distinction —
    /// `None` is **not** a fail-open marker, it means "skip the
    /// per-extension allowlist step". Mutated only via
    /// [`Self::insert_template`] so `by_name` and `owners` cannot
    /// drift.
    owners: BTreeMap<String, ExtensionId>,
}

impl TemplateRegistry {
    /// Construct an empty registry.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Construct the host-builtin registry — the four templates
    /// `AnalyticsBridge` currently hard-codes, lifted into spec form.
    ///
    /// The SQL bodies are captured verbatim (with `$tenant_id` /
    /// `$meter_id` placeholders for the bound parameters and
    /// `$bucket` / `$window` for the time-bucket arguments the
    /// resolver inlines from the matched template). They are
    /// **descriptive**: the host's resolver matches templates by
    /// name and runs the corresponding `sqlx::query_as` call; the
    /// SQL string here exists so operators can audit the catalog
    /// (`describe` / a future admin endpoint reads it).
    pub fn builtin() -> Self {
        Self::empty()
            .with(TemplateSpec {
                name: "meter_kwh_last_24h".to_string(),
                params: serde_json::json!({
                    "type": "object",
                    "required": ["tenant_id"],
                    "properties": {
                        "tenant_id": { "type": "string" }
                    }
                }),
                tables: vec!["samples".to_string()],
                sql: Some(
                    "SELECT value_num AS kwh \
                     FROM samples \
                     WHERE tenant_id = $tenant_id \
                       AND entity_id LIKE $tenant_id || '.elec.%' \
                       AND value_num IS NOT NULL \
                     ORDER BY ts DESC LIMIT 1"
                        .to_string(),
                ),
            })
            .with(TemplateSpec {
                name: "meter_litres_last_24h".to_string(),
                params: serde_json::json!({
                    "type": "object",
                    "required": ["tenant_id"],
                    "properties": {
                        "tenant_id": { "type": "string" }
                    }
                }),
                tables: vec!["samples".to_string()],
                sql: Some(
                    "SELECT value_num AS litres \
                     FROM samples \
                     WHERE tenant_id = $tenant_id \
                       AND entity_id LIKE $tenant_id || '.water.%' \
                       AND value_num IS NOT NULL \
                     ORDER BY ts DESC LIMIT 1"
                        .to_string(),
                ),
            })
            .with(TemplateSpec {
                name: "meter_value_30d_15m".to_string(),
                params: serde_json::json!({
                    "type": "object",
                    "required": ["tenant_id", "meter_id"],
                    "properties": {
                        "tenant_id": { "type": "string" },
                        "meter_id":  { "type": "string" }
                    }
                }),
                tables: vec!["samples".to_string()],
                sql: Some(
                    "SELECT time_bucket('15 minutes', ts) AS bucket_start, \
                            AVG(value_num) AS value_avg \
                     FROM samples \
                     WHERE tenant_id = $tenant_id \
                       AND entity_id = $meter_id \
                       AND ts >= NOW() - '30 days'::interval \
                       AND value_num IS NOT NULL \
                     GROUP BY bucket_start ORDER BY bucket_start ASC"
                        .to_string(),
                ),
            })
            .with(TemplateSpec {
                name: "meter_value_24h_1m".to_string(),
                params: serde_json::json!({
                    "type": "object",
                    "required": ["tenant_id", "meter_id"],
                    "properties": {
                        "tenant_id": { "type": "string" },
                        "meter_id":  { "type": "string" }
                    }
                }),
                tables: vec!["samples".to_string()],
                sql: Some(
                    "SELECT time_bucket('1 minute', ts) AS bucket_start, \
                            AVG(value_num) AS value_avg \
                     FROM samples \
                     WHERE tenant_id = $tenant_id \
                       AND entity_id = $meter_id \
                       AND ts >= NOW() - '24 hours'::interval \
                       AND value_num IS NOT NULL \
                     GROUP BY bucket_start ORDER BY bucket_start ASC"
                        .to_string(),
                ),
            })
    }

    /// Builder-style insert. Replaces any existing entry with the
    /// same name.
    pub fn with(mut self, spec: TemplateSpec) -> Self {
        self.insert(spec);
        self
    }

    /// Mutable insert. Replaces any existing entry with the same name
    /// (extensions contributing a template that collides with a
    /// builtin shadow the builtin — host loader enforces this is
    /// only legal when an explicit `override:` flag lands in the
    /// manifest, which is a row-3 follow-up; row 2 silently
    /// overwrites).
    ///
    /// This path leaves no owner attached — equivalent to a builtin
    /// insert. Use [`Self::extend_from_record`] for
    /// extension-contributed templates so the owning
    /// [`ExtensionId`] is recorded for the SDUI allowlist gate.
    pub fn insert(&mut self, spec: TemplateSpec) {
        let name = spec.name.clone();
        self.insert_template(name, spec, None);
    }

    /// Single insertion choke point. Every path that registers a
    /// template — [`Self::builtin`], [`Self::extend_from_record`],
    /// and the builder-style [`Self::with`] / [`Self::insert`] —
    /// funnels through here so `by_name` and `owners` can never
    /// drift. Builtins pass `owner = None`; extension-contributed
    /// templates pass `Some(ext_id)`.
    fn insert_template(&mut self, name: String, spec: TemplateSpec, owner: Option<ExtensionId>) {
        self.by_name.insert(name.clone(), spec);
        match owner {
            Some(id) => {
                self.owners.insert(name, id);
            }
            None => {
                // A builtin (or `insert`) shadowing a previously
                // contributed entry must also clear the owner so the
                // SDUI gate does not later try to intersect a stale
                // extension's grant against the builtin's tables.
                self.owners.remove(&name);
            }
        }
    }

    /// Look up the extension that contributed a template by name.
    ///
    /// Returns `None` for host-builtin templates (those registered
    /// via [`Self::builtin`] / [`Self::insert`] / [`Self::with`]) —
    /// callers must read `None` as **"host-trusted, no
    /// per-extension allowlist to apply"**, not as a fail-open
    /// signal. The SDUI bridge's per-call allowlist gate uses this
    /// distinction: `Some(ext_id)` means intersect `spec.tables`
    /// with the extension's `contributes.warehouse_tables[]` grant;
    /// `None` means skip the intersection.
    pub fn owning_extension(&self, name: &str) -> Option<&ExtensionId> {
        self.owners.get(name)
    }

    /// Lookup by name.
    pub fn get(&self, name: &str) -> Option<&TemplateSpec> {
        self.by_name.get(name)
    }

    /// `true` if the registry contains a template by that name.
    pub fn contains(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// Iterate all registered templates in name order (the underlying
    /// store is a `BTreeMap`).
    pub fn iter(&self) -> impl Iterator<Item = &TemplateSpec> {
        self.by_name.values()
    }

    /// Number of registered templates.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// `true` when no templates are registered.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Read every `contributes.warehouse_templates[]` entry from
    /// `record` and insert the resulting [`TemplateSpec`]s into the
    /// registry.
    ///
    /// For each entry:
    /// - reads `params_schema` from the bundle directory and parses
    ///   it as JSON (the file must be a syntactically valid JSON
    ///   Schema fragment — schema *meaning* is not validated here);
    /// - reads `sql_file` if present, storing the body verbatim;
    /// - inserts the [`TemplateSpec`] under its `name`.
    ///
    /// Returns the number of templates inserted. Records with no
    /// manifest (e.g. `Failed`) or no contributed templates produce
    /// `Ok(0)` and do not touch the registry.
    ///
    /// **Shadowing rule.** A contributed template whose name matches
    /// an existing entry (builtin or another extension's contribution)
    /// silently replaces it — this is the row-3 baseline behaviour
    /// matching every other contribute slice. A `override:` opt-in
    /// flag plus a hard collision-error path is a follow-up listed in
    /// the row-3 design doc as the next sharp edge.
    ///
    /// I/O is bounded to the bundle directory: every path is resolved
    /// via `bundle_dir.join(...)`; the loader rejects absolute paths
    /// and `..` segments at the consumer-controlled extension root,
    /// matching how every other contribute slice resolves its file
    /// references.
    pub fn extend_from_record(&mut self, record: &ExtensionRecord) -> Result<usize, Error> {
        let Some(manifest) = record.manifest.as_ref() else {
            return Ok(0);
        };
        // `manifest.id` is the validated extension id; prefer it over
        // `record.id` to avoid the (rare) early-failure case where
        // `record.id` was not yet set.
        let owner = record.id.clone().unwrap_or_else(|| manifest.id.clone());
        let mut inserted = 0;
        for entry in &manifest.contributes.warehouse_templates {
            let spec = load_template_spec(&record.bundle_dir, entry)?;
            // Silent-capture warning: contributed templates that
            // overwrite an existing entry (builtin or another
            // extension's) are loaded but logged. Hard rejection is
            // deferred to an explicit `override:` opt-in (see the
            // row-3 design doc) — flipping the default to reject
            // would force every existing manifest through a
            // migration.
            if self.by_name.contains_key(&entry.name) {
                let previous_owner = self
                    .owners
                    .get(&entry.name)
                    .map(|i| i.as_str().to_string())
                    .unwrap_or_else(|| "<builtin>".into());
                warn!(
                    target: "starter_ext_host.warehouse",
                    template = %entry.name,
                    new_owner = %owner.as_str(),
                    previous_owner = %previous_owner,
                    "warehouse template name collision — overwriting existing entry",
                );
            }
            self.insert_template(entry.name.clone(), spec, Some(owner.clone()));
            inserted += 1;
        }
        Ok(inserted)
    }
}

/// Resolve `entry`'s `params_schema` (and optional `sql_file`)
/// against `bundle_dir` and build the [`TemplateSpec`].
fn load_template_spec(
    bundle_dir: &Path,
    entry: &ContributeWarehouseTemplate,
) -> Result<TemplateSpec, Error> {
    let schema_path = bundle_dir.join(&entry.params_schema);
    let schema_bytes = std::fs::read(&schema_path).map_err(|e| {
        Error::manifest(format!(
            "warehouse template {:?}: failed to read params_schema {}: {}",
            entry.name,
            schema_path.display(),
            e
        ))
    })?;
    let params: serde_json::Value = serde_json::from_slice(&schema_bytes).map_err(|e| {
        Error::manifest(format!(
            "warehouse template {:?}: params_schema {} is not valid JSON: {}",
            entry.name,
            schema_path.display(),
            e
        ))
    })?;
    let sql = match entry.sql_file.as_deref() {
        None => None,
        Some(rel) => {
            let p = bundle_dir.join(rel);
            let body = std::fs::read_to_string(&p).map_err(|e| {
                Error::manifest(format!(
                    "warehouse template {:?}: failed to read sql_file {}: {}",
                    entry.name,
                    p.display(),
                    e
                ))
            })?;
            Some(body)
        }
    };
    Ok(TemplateSpec {
        name: entry.name.clone(),
        params,
        tables: entry.tables.clone(),
        sql,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::manifest::{
        ContributeWarehouseTemplate, Contributes, Manifest, Runtime, RuntimeKind,
    };
    use starter_ext_spi::{ExtensionId, LifecycleState};
    use std::path::PathBuf;

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, body).unwrap();
    }

    fn make_record(
        bundle: PathBuf,
        ext_id: &str,
        t: ContributeWarehouseTemplate,
    ) -> ExtensionRecord {
        let id = ExtensionId::new(ext_id).unwrap();
        let manifest = Manifest {
            v: 1,
            id: id.clone(),
            version: semver::Version::new(0, 1, 0),
            display_name: ext_id.into(),
            description_file: None,
            authors: vec![],
            requires: vec![],
            requires_extensions: vec![],
            runtime: Runtime {
                kind: RuntimeKind::Builtin,
                bin: None,
                crate_name: None,
                artefact: None,
            },
            supervision: None,
            capabilities: vec![],
            config_schema: None,
            config: serde_json::Value::Null,
            contributes: Contributes {
                warehouse_templates: vec![t],
                ..Contributes::default()
            },
        };
        ExtensionRecord {
            id: Some(id),
            id_hint: ext_id.into(),
            bundle_dir: bundle,
            state: LifecycleState::Validated,
            manifest: Some(manifest),
            failure: None,
            origin: crate::BundleOrigin::default(),
        }
    }

    #[test]
    fn builtin_registers_four_templates() {
        let r = TemplateRegistry::builtin();
        assert_eq!(r.len(), 4);
        for name in [
            "meter_kwh_last_24h",
            "meter_litres_last_24h",
            "meter_value_30d_15m",
            "meter_value_24h_1m",
        ] {
            let spec = r.get(name).unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(spec.tables, vec!["samples"]);
            assert!(spec.sql.as_ref().unwrap().contains("samples"));
        }
    }

    #[test]
    fn unknown_template_returns_none() {
        let r = TemplateRegistry::builtin();
        assert!(r.get("nope").is_none());
        assert!(!r.contains("nope"));
    }

    #[test]
    fn with_replaces_existing_entry() {
        let r = TemplateRegistry::builtin().with(TemplateSpec {
            name: "meter_kwh_last_24h".into(),
            params: serde_json::json!({}),
            tables: vec!["other".into()],
            sql: None,
        });
        assert_eq!(r.len(), 4);
        let spec = r.get("meter_kwh_last_24h").unwrap();
        assert_eq!(spec.tables, vec!["other"]);
        assert!(spec.sql.is_none());
    }

    #[test]
    fn iter_yields_name_sorted() {
        let r = TemplateRegistry::builtin();
        let names: Vec<&str> = r.iter().map(|s| s.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn extend_from_record_reads_schema_and_sql() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "schemas/q.json",
            r#"{"type":"object","required":["tenant_id"]}"#,
        );
        write(tmp.path(), "sql/q.sql", "SELECT 1 FROM samples");
        let mut r = TemplateRegistry::empty();
        let rec = make_record(
            tmp.path().to_path_buf(),
            "com.acme.charts",
            ContributeWarehouseTemplate {
                name: "com.acme.charts.q".into(),
                params_schema: "schemas/q.json".into(),
                tables: vec!["samples".into()],
                sql_file: Some("sql/q.sql".into()),
            },
        );
        let n = r.extend_from_record(&rec).unwrap();
        assert_eq!(n, 1);
        let spec = r.get("com.acme.charts.q").unwrap();
        assert_eq!(spec.tables, vec!["samples"]);
        assert_eq!(spec.sql.as_deref(), Some("SELECT 1 FROM samples"));
        assert_eq!(spec.params["type"], "object");
    }

    #[test]
    fn extend_from_record_optional_sql() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "s.json", "{}");
        let mut r = TemplateRegistry::empty();
        let rec = make_record(
            tmp.path().to_path_buf(),
            "com.acme.charts",
            ContributeWarehouseTemplate {
                name: "com.acme.charts.q".into(),
                params_schema: "s.json".into(),
                tables: vec![],
                sql_file: None,
            },
        );
        assert_eq!(r.extend_from_record(&rec).unwrap(), 1);
        assert!(r.get("com.acme.charts.q").unwrap().sql.is_none());
    }

    #[test]
    fn extend_from_record_missing_schema_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let mut r = TemplateRegistry::empty();
        let rec = make_record(
            tmp.path().to_path_buf(),
            "com.acme.charts",
            ContributeWarehouseTemplate {
                name: "com.acme.charts.q".into(),
                params_schema: "nope.json".into(),
                tables: vec![],
                sql_file: None,
            },
        );
        let err = r.extend_from_record(&rec).unwrap_err();
        assert!(err.to_string().contains("params_schema"), "{err}");
    }

    #[test]
    fn extend_from_record_invalid_json_errors() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "s.json", "not json");
        let mut r = TemplateRegistry::empty();
        let rec = make_record(
            tmp.path().to_path_buf(),
            "com.acme.charts",
            ContributeWarehouseTemplate {
                name: "com.acme.charts.q".into(),
                params_schema: "s.json".into(),
                tables: vec![],
                sql_file: None,
            },
        );
        let err = r.extend_from_record(&rec).unwrap_err();
        assert!(err.to_string().contains("not valid JSON"), "{err}");
    }

    #[test]
    fn owning_extension_returns_owner_for_contributed_template() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "s.json", "{}");
        write(tmp.path(), "q.sql", "SELECT 1");
        let mut r = TemplateRegistry::empty();
        let rec = make_record(
            tmp.path().to_path_buf(),
            "com.acme.charts",
            ContributeWarehouseTemplate {
                name: "com.acme.charts.q".into(),
                params_schema: "s.json".into(),
                tables: vec!["t".into()],
                sql_file: Some("q.sql".into()),
            },
        );
        r.extend_from_record(&rec).unwrap();
        let owner = r.owning_extension("com.acme.charts.q").unwrap();
        assert_eq!(owner.as_str(), "com.acme.charts");
        // Builtins have no owner.
        let b = TemplateRegistry::builtin();
        assert!(b.owning_extension("meter_kwh_last_24h").is_none());
    }

    #[test]
    fn insert_clears_owner_when_shadowing_contributed() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "s.json", "{}");
        let mut r = TemplateRegistry::empty();
        let rec = make_record(
            tmp.path().to_path_buf(),
            "com.acme.charts",
            ContributeWarehouseTemplate {
                name: "com.acme.charts.q".into(),
                params_schema: "s.json".into(),
                tables: vec![],
                sql_file: None,
            },
        );
        r.extend_from_record(&rec).unwrap();
        assert!(r.owning_extension("com.acme.charts.q").is_some());
        // Insert a plain (builtin-shaped) entry — owner must clear.
        r.insert(TemplateSpec {
            name: "com.acme.charts.q".into(),
            params: serde_json::json!({}),
            tables: vec![],
            sql: None,
        });
        assert!(r.owning_extension("com.acme.charts.q").is_none());
    }

    #[test]
    fn extend_from_record_collision_logs_but_loads() {
        // Two records contributing the same template name; second
        // insert overwrites silently (warn-only). The second owner
        // wins.
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "s.json", "{}");
        let mut r = TemplateRegistry::empty();
        let rec1 = make_record(
            tmp.path().to_path_buf(),
            "com.acme.first",
            ContributeWarehouseTemplate {
                name: "com.acme.first.q".into(),
                params_schema: "s.json".into(),
                tables: vec!["a".into()],
                sql_file: None,
            },
        );
        r.extend_from_record(&rec1).unwrap();
        // Use the same name on the second record (validate_manifest
        // would reject the namespace-mismatch, but the registry
        // itself is the layer under test).
        let mut rec2 = make_record(
            tmp.path().to_path_buf(),
            "com.acme.first",
            ContributeWarehouseTemplate {
                name: "com.acme.first.q".into(),
                params_schema: "s.json".into(),
                tables: vec!["b".into()],
                sql_file: None,
            },
        );
        // Pretend it came from a different extension to exercise the
        // owner-change branch of the collision warning.
        rec2.id = Some(ExtensionId::new("com.acme.second").unwrap());
        if let Some(m) = rec2.manifest.as_mut() {
            m.id = ExtensionId::new("com.acme.second").unwrap();
        }
        r.extend_from_record(&rec2).unwrap();
        let spec = r.get("com.acme.first.q").unwrap();
        assert_eq!(spec.tables, vec!["b"]);
        assert_eq!(
            r.owning_extension("com.acme.first.q").unwrap().as_str(),
            "com.acme.second"
        );
    }

    #[test]
    fn extend_from_record_no_manifest_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let mut r = TemplateRegistry::empty();
        let rec = ExtensionRecord {
            id: None,
            id_hint: "x".into(),
            bundle_dir: tmp.path().to_path_buf(),
            state: LifecycleState::Failed,
            manifest: None,
            failure: None,
            origin: crate::BundleOrigin::default(),
        };
        assert_eq!(r.extend_from_record(&rec).unwrap(), 0);
        assert!(r.is_empty());
    }
}
