//! `TimescaleAnalyticsBridge` — implements
//! [`starter_sdui_routes::AnalyticsBridge`] against the Timescale
//! `samples` hypertable.
//!
//! The four named templates the bundled `data-flow-site-a`
//! dashboard uses are resolved by
//! [`super::template_resolver::resolve`]. That module is the
//! single source of truth shared with
//! [`crate::extensions::backends::RubixWarehouseReadBackend`] —
//! adding or removing a template is a one-place change.
//!
//! Templates outside the resolver's known set return an empty row
//! vector — the upstream resolver then renders the chart / KPI as
//! no-data, which is the same outcome as having no bridge at all.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use starter_ext_host::{ExtensionRegistry, TemplateRegistry};
use starter_sdui_routes::AnalyticsBridge;
use starter_store_warehouse::WarehouseClient;
use tracing::warn;

use super::template_resolver;
use crate::extensions::backends::warehouse_tables_for;

/// Concrete bridge backed by a Timescale `samples` hypertable.
///
/// The bridge consults [`TemplateRegistry`] for name resolution — the
/// registry is the audit source of truth across host-builtin
/// templates and (once row 3 lands) extension-contributed ones.
/// Per-template SQL lives in [`super::template_resolver`].
#[derive(Clone)]
pub struct TimescaleAnalyticsBridge {
    client: WarehouseClient,
    registry: Arc<TemplateRegistry>,
    /// Optional view onto the `ExtensionRegistry`. When present, the
    /// per-call gate intersects each contributed template's declared
    /// tables with the owning extension's
    /// `contributes.warehouse_tables[]` grant — same shape as the
    /// host-methods path. When absent, the gate is bypassed for all
    /// templates (development/test bring-up); in production this is
    /// always wired by `boot::build_sdui_router`.
    extension_registry: Option<Arc<ExtensionRegistry>>,
}

impl TimescaleAnalyticsBridge {
    /// Construct with the host-builtin template registry. The four
    /// templates the bridge resolves are registered as builtins via
    /// [`TemplateRegistry::builtin`].
    pub fn new(client: WarehouseClient) -> Self {
        Self::with_registry(client, Arc::new(TemplateRegistry::builtin()))
    }

    /// Construct with an explicit registry. Used by tests and by host
    /// integrations that wire extension-contributed templates on top
    /// of the builtin set.
    pub fn with_registry(client: WarehouseClient, registry: Arc<TemplateRegistry>) -> Self {
        Self {
            client,
            registry,
            extension_registry: None,
        }
    }

    /// Attach the host's `ExtensionRegistry` so the per-call
    /// `warehouse_tables_for(...)` allowlist gate can resolve the
    /// caller's grant. Threaded through `build_sdui_router` from
    /// `compose.rs` alongside the template registry.
    pub fn with_extension_registry(mut self, registry: Arc<ExtensionRegistry>) -> Self {
        self.extension_registry = Some(registry);
        self
    }

    /// Borrow the underlying warehouse client. Surfaced so the
    /// extension-substrate backend factory can share the same pool
    /// without re-plumbing config.
    pub fn client(&self) -> &WarehouseClient {
        &self.client
    }

    /// Borrow the registry. Surfaced for the same reason as
    /// [`Self::client`].
    pub fn registry(&self) -> &Arc<TemplateRegistry> {
        &self.registry
    }
}

#[async_trait]
impl AnalyticsBridge for TimescaleAnalyticsBridge {
    async fn invoke(
        &self,
        name: &str,
        params: &BTreeMap<String, JsonValue>,
    ) -> Result<Vec<JsonValue>, String> {
        // Catalog gate: an unknown template is refused regardless of
        // whether a resolver would match. This makes the
        // `TemplateRegistry` the single audit source of truth —
        // adding a host-builtin template or accepting a contributed
        // one is the only path to a new resolvable name.
        if self.registry.get(name).is_none() {
            warn!(
                target: "rubix.sdui.analytics_bridge",
                template = name,
                "unknown analytics template (not in TemplateRegistry); returning empty",
            );
            return Ok(vec![]);
        }
        let spec = self.registry.get(name);

        // Per-call table allowlist gate. Mirrors the host-methods
        // path (`RubixWarehouseReadBackend`): a contributed template
        // may only read tables present in its owning extension's
        // `contributes.warehouse_tables[]` grant. `None` from
        // `owning_extension` means **builtin — host-trusted; skip
        // the gate**; this is *not* a fail-open (the four
        // `meter_*` builtins legitimately read `samples`, which no
        // extension contributes).
        if let (Some(spec), Some(owner)) = (spec, self.registry.owning_extension(name)) {
            let granted = warehouse_tables_for(self.extension_registry.as_deref(), owner);
            let granted_names: std::collections::BTreeSet<&str> =
                granted.iter().map(|g| g.name.as_str()).collect();
            for table in &spec.tables {
                if !granted_names.contains(table.as_str()) {
                    return Err(format!(
                        "{name}: template references table {table:?} not in \
                         owning extension {:?}'s warehouse_tables[] grant",
                        owner.as_str()
                    ));
                }
            }
        }

        let Some(tenant_id) = params.get("tenant_id").and_then(|v| v.as_str()) else {
            return Err(format!("{name}: tenant_id required"));
        };

        // Hand the rest off to the shared resolver. The BTreeMap
        // shape the SDUI layer uses is structurally compatible with
        // a JSON object; serialise once and pass the value through.
        let params_json: JsonValue = json!(params);
        template_resolver::resolve(&self.client, name, tenant_id, &params_json, spec).await
    }
}

#[cfg(test)]
mod tests {
    //! Stage-1 gate tests for the bridge. Resolution of real SQL
    //! requires a live warehouse and lives in the integration
    //! suite; these tests cover the per-call allowlist gate only —
    //! it short-circuits before the warehouse call so we can use
    //! a lazy pool that never connects.
    use super::*;
    use starter_ext_host::ExtensionRegistry;
    use starter_ext_spi::manifest::{
        ContributeWarehouseTable, ContributeWarehouseTemplate, Contributes, Manifest, Runtime,
        RuntimeKind, TableColumn,
    };
    use starter_ext_spi::{ExtensionId, LifecycleState};
    use starter_ext_host::record::ExtensionRecord;

    fn lazy_client() -> WarehouseClient {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
            .expect("connect_lazy infallible for valid URLs");
        WarehouseClient::from_pool(pool)
    }

    fn make_record(ext: &str, tables: Vec<&str>) -> ExtensionRecord {
        let id = ExtensionId::new(ext).unwrap();
        let manifest = Manifest {
            v: 1,
            id: id.clone(),
            version: semver::Version::new(0, 1, 0),
            display_name: ext.into(),
            description_file: None,
            authors: vec![],
            requires: vec![],
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
                warehouse_tables: tables
                    .into_iter()
                    .map(|t| ContributeWarehouseTable {
                        name: t.into(),
                        columns: vec![TableColumn {
                            name: "v".into(),
                            ty: "TEXT".into(),
                            default: None,
                        }],
                        order_by: vec!["v".into()],
                        engine: None,
                        partition_by: None,
                        ttl: None,
                    })
                    .collect(),
                ..Contributes::default()
            },
        };
        ExtensionRecord {
            id: Some(id),
            id_hint: ext.into(),
            bundle_dir: std::path::PathBuf::new(),
            state: LifecycleState::Validated,
            manifest: Some(manifest),
            failure: None,
            origin: starter_ext_host::BundleOrigin::default(),
        }
    }

    fn ext_registry(rec: ExtensionRecord) -> Arc<ExtensionRegistry> {
        let mut reg = ExtensionRegistry::new();
        let mut map = std::collections::HashMap::new();
        map.insert(rec.id.as_ref().unwrap().as_str().to_string(), rec);
        reg.install(map);
        reg.seal();
        Arc::new(reg)
    }

    #[tokio::test]
    async fn gate_rejects_when_table_not_in_grant() {
        let mut tmpl = TemplateRegistry::empty();
        // Build a contributed template referencing table `secret_t`
        // while the extension's grant lists only `allowed_t` — gate
        // must reject.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("s.json"), "{}").unwrap();
        let id = ExtensionId::new("com.acme.ext").unwrap();
        let mfst = Manifest {
            v: 1,
            id: id.clone(),
            version: semver::Version::new(0, 1, 0),
            display_name: "x".into(),
            description_file: None,
            authors: vec![],
            requires: vec![],
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
                warehouse_templates: vec![ContributeWarehouseTemplate {
                    name: "com.acme.ext.q".into(),
                    params_schema: "s.json".into(),
                    tables: vec!["secret_t".into()],
                    sql_file: None,
                }],
                ..Contributes::default()
            },
        };
        let rec_for_tmpl = ExtensionRecord {
            id: Some(id.clone()),
            id_hint: "com.acme.ext".into(),
            bundle_dir: tmp.path().to_path_buf(),
            state: LifecycleState::Validated,
            manifest: Some(mfst),
            failure: None,
            origin: starter_ext_host::BundleOrigin::default(),
        };
        tmpl.extend_from_record(&rec_for_tmpl).unwrap();

        // ExtensionRegistry granting only `allowed_t`.
        let rec = make_record("com.acme.ext", vec!["allowed_t"]);
        let bridge = TimescaleAnalyticsBridge::with_registry(lazy_client(), Arc::new(tmpl))
            .with_extension_registry(ext_registry(rec));

        let mut params = BTreeMap::new();
        params.insert("tenant_id".into(), serde_json::json!("t1"));
        let err = bridge
            .invoke("com.acme.ext.q", &params)
            .await
            .unwrap_err();
        assert!(err.contains("not in") && err.contains("secret_t"), "{err}");
    }

    #[tokio::test]
    async fn gate_bypasses_builtin_templates() {
        // Builtins have no owner; gate must skip the intersection
        // step entirely. We can't actually run SQL here, but we
        // can confirm the gate doesn't reject — calling invoke on
        // a builtin name with the gate path is what we exercise.
        // The bridge will try to resolve and fail at the DB layer;
        // we only assert the gate didn't short-circuit with the
        // allowlist-shaped error.
        let tmpl = TemplateRegistry::builtin();
        let rec = make_record("com.acme.ext", vec![]);
        let bridge = TimescaleAnalyticsBridge::with_registry(lazy_client(), Arc::new(tmpl))
            .with_extension_registry(ext_registry(rec));
        let mut params = BTreeMap::new();
        params.insert("tenant_id".into(), serde_json::json!("t1"));
        let res = bridge.invoke("meter_kwh_last_24h", &params).await;
        // Either Ok (impossible without DB) or an error that is NOT
        // the allowlist-gate error — anything else proves the gate
        // ran. We assert by string: gate's message contains
        // "not in" and "warehouse_tables[]".
        if let Err(e) = res {
            assert!(
                !e.contains("warehouse_tables[]"),
                "gate must not fire on builtin: {e}"
            );
        }
    }

    #[tokio::test]
    async fn gate_accepts_when_tables_subset_of_grant() {
        let mut tmpl = TemplateRegistry::empty();
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("s.json"), "{}").unwrap();
        let id = ExtensionId::new("com.acme.ext").unwrap();
        let mfst = Manifest {
            v: 1,
            id: id.clone(),
            version: semver::Version::new(0, 1, 0),
            display_name: "x".into(),
            description_file: None,
            authors: vec![],
            requires: vec![],
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
                warehouse_templates: vec![ContributeWarehouseTemplate {
                    name: "com.acme.ext.q".into(),
                    params_schema: "s.json".into(),
                    tables: vec!["allowed_t".into()],
                    sql_file: None,
                }],
                ..Contributes::default()
            },
        };
        let rec_for_tmpl = ExtensionRecord {
            id: Some(id.clone()),
            id_hint: "com.acme.ext".into(),
            bundle_dir: tmp.path().to_path_buf(),
            state: LifecycleState::Validated,
            manifest: Some(mfst),
            failure: None,
            origin: starter_ext_host::BundleOrigin::default(),
        };
        tmpl.extend_from_record(&rec_for_tmpl).unwrap();

        let rec = make_record("com.acme.ext", vec!["allowed_t"]);
        let bridge = TimescaleAnalyticsBridge::with_registry(lazy_client(), Arc::new(tmpl))
            .with_extension_registry(ext_registry(rec));
        let mut params = BTreeMap::new();
        params.insert("tenant_id".into(), serde_json::json!("t1"));
        // Gate accepts; resolver then fails on the lazy pool (no DB).
        // What we assert: error, if any, is NOT the gate message.
        if let Err(e) = bridge.invoke("com.acme.ext.q", &params).await {
            assert!(
                !e.contains("warehouse_tables[] grant"),
                "gate should not reject when table is granted: {e}"
            );
        }
    }
}
