//! End-to-end gate for `WarehouseTableKind::ContinuousAggregate`.
//!
//! Drives `boot::create_extension_tables` against a real Timescale
//! container with a synthetic manifest that mixes one plain
//! `kind: table` entry and one `kind: continuous_aggregate` entry,
//! then queries `pg_class` to prove the CAGG-flagged entry produced
//! **no** relation while the plain entry produced one. This is the
//! guarantee the per-call allowlist gate depends on: registering a
//! continuous-aggregate name must not stamp an empty stub that
//! would later block the extension's `scripts/post-load.sql` from
//! installing the materialised view.
//!
//! Ignored by default — needs Docker (testcontainers Timescale).
//! Run with `--ignored` under the integration job.

use std::sync::Arc;

use rubix_agent::boot::create_extension_tables;
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_spi::manifest::WarehouseTableKind;
use starter_ext_spi::LifecycleState;
use starter_store_warehouse::testing::with_timescale;

const EXT_ID: &str = "com.acme.warehouse_kinds";

/// One `kind: table` entry (`raw`) + one `kind: continuous_aggregate`
/// entry (`rollup_1m`). `runtime.kind: builtin` keeps the loader off
/// the filesystem for everything except the YAML itself.
const FIXTURE_YAML: &str = r#"v: 1
id: com.acme.warehouse_kinds
version: 0.1.0
display_name: "Warehouse-kinds gate fixture"
runtime: { kind: builtin, crate_name: warehouse_kinds }
contributes:
  warehouse_tables:
    - name: raw
      order_by: [ts]
      columns:
        - { name: ts,    type: "TIMESTAMPTZ" }
        - { name: value, type: "DOUBLE PRECISION", default: "NULL" }
    - name: rollup_1m
      kind: continuous_aggregate
      order_by: [bucket]
      columns:
        - { name: bucket,    type: "TIMESTAMPTZ" }
        - { name: avg_value, type: "DOUBLE PRECISION", default: "NULL" }
"#;

fn write_fixture_bundle() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let bundle = dir.path().join(EXT_ID);
    std::fs::create_dir_all(&bundle).expect("create bundle dir");
    std::fs::write(bundle.join("block.yaml"), FIXTURE_YAML).expect("write block.yaml");
    dir
}

fn load_fixture() -> (Arc<ExtensionRegistry>, tempfile::TempDir) {
    let dir = write_fixture_bundle();
    let records = Loader::scan(dir.path()).validate_all();
    assert_eq!(records.len(), 1, "expected exactly one fixture bundle");
    assert_eq!(
        records[0].state,
        LifecycleState::Validated,
        "fixture manifest failed validation: {:?}",
        records[0].failure,
    );
    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(records, &mut registry);
    assert_eq!(outcome.validated, 1);
    assert_eq!(outcome.failed, 0);
    registry.seal();
    (Arc::new(registry), dir)
}

async fn relation_exists(pool: &sqlx::PgPool, name: &str) -> bool {
    let row: (Option<String>,) = sqlx::query_as("SELECT to_regclass($1)::text")
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("to_regclass query");
    row.0.is_some()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Docker (testcontainers Timescale); run via the integration job"]
async fn cagg_kind_skips_host_ddl_while_table_kind_creates() {
    let (client, _guard) = with_timescale().await;
    let (registry, _dir) = load_fixture();

    let outcome = create_extension_tables(&registry, &client).await;

    assert_eq!(outcome.seen, 2, "saw both manifest entries");
    assert_eq!(
        outcome.created_or_existing, 1,
        "exactly one entry should be host-managed (the kind: table)"
    );
    assert_eq!(
        outcome.deferred_to_extension, 1,
        "the kind: continuous_aggregate entry must defer to the extension"
    );
    assert_eq!(outcome.skipped, 0, "no validation failures expected");

    // Manifest-side: the plain table is created with the host's
    // namespacing scheme, so it's queryable through `to_regclass`.
    let raw_name = "public.com_acme_warehouse_kinds__raw";
    assert!(
        relation_exists(client.pool(), raw_name).await,
        "{raw_name} must exist after host DDL"
    );

    // The CAGG-flagged entry must NOT exist as any relation. This is
    // the actual race we're defending against: if the host had
    // created an empty stub here, an extension's later
    // `CREATE MATERIALIZED VIEW IF NOT EXISTS` would silently no-op
    // and dashboards would read empty results forever.
    let cagg_name = "public.com_acme_warehouse_kinds__rollup_1m";
    assert!(
        !relation_exists(client.pool(), cagg_name).await,
        "{cagg_name} must NOT be created by the host \
         (extension owns its DDL via post-load.sql)"
    );
}

/// Sanity: the real `com.nubeio.rubixos` bundle round-trips through
/// the loader with the new `kind: continuous_aggregate` flag
/// preserved. Pure parse — no Docker — so it runs in the default
/// test pass and catches a regression in the YAML / serde schema
/// without needing the integration job. Pinned to the workspace
/// path; runs from any cwd under the workspace because the
/// `CARGO_MANIFEST_DIR` of `rubix-agent` is two levels deep.
#[test]
fn nubeio_block_yaml_preserves_continuous_aggregate_kind() {
    let manifest_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extensions");
    if !manifest_dir.exists() {
        // Workspace layout changed; surface that clearly rather
        // than silently passing.
        panic!(
            "expected extensions dir at {} (workspace layout drift)",
            manifest_dir.display()
        );
    }
    let records = Loader::scan(&manifest_dir).validate_all();
    let nubeio = records
        .iter()
        .find(|r| {
            r.id.as_ref()
                .map(|i| i.as_str() == "com.nubeio.rubixos")
                .unwrap_or(false)
        })
        .expect("com.nubeio.rubixos bundle not picked up by Loader::scan");
    assert_eq!(
        nubeio.state,
        LifecycleState::Validated,
        "real block.yaml failed validation: {:?}",
        nubeio.failure,
    );
    let manifest = nubeio.manifest.as_ref().expect("manifest present");

    let histories_1m = manifest
        .contributes
        .warehouse_tables
        .iter()
        .find(|t| t.name == "histories_1m")
        .expect("histories_1m entry missing from manifest");
    assert_eq!(
        histories_1m.kind,
        WarehouseTableKind::ContinuousAggregate,
        "histories_1m must be flagged as a continuous aggregate \
         so the host skips DDL and post-load.sql owns creation"
    );
    assert!(!histories_1m.kind.host_manages_ddl());

    let histories = manifest
        .contributes
        .warehouse_tables
        .iter()
        .find(|t| t.name == "histories")
        .expect("histories entry missing from manifest");
    assert_eq!(
        histories.kind,
        WarehouseTableKind::Table,
        "plain tables must default to kind: table"
    );
    assert!(histories.kind.host_manages_ddl());
}
