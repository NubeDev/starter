//! Integration tests that need a live ClickHouse + Postgres
//! stack via testcontainers. All `#[ignore]` so the default
//! `cargo test -p starter-warehouse` is fast. Run via:
//!
//! ```text
//! cargo test -p starter-warehouse --features 'warehouse testing' -- --ignored
//! ```
//!
//! Each test names the W-rule it exercises so the REVIEW gate 2
//! transcript reads as a one-to-one map.

#![cfg(all(feature = "warehouse", feature = "testing"))]

use std::sync::Arc;

use chrono::{Duration, Utc};
use starter_store_postgres::testing::with_database;
use starter_store_warehouse::testing::with_clickhouse;
use starter_tags::TagQuery;
use std::str::FromStr;

use starter_warehouse::catalog::mart_spec::{AggregationSpec, MartSpec};
use starter_warehouse::nodes::runtime::{RuntimeError, WarehouseRuntime};
use starter_warehouse::WarehouseConfig;

fn sample_spec(name: &str, created_by: &str) -> MartSpec {
    MartSpec {
        name: name.into(),
        description: None,
        source_table: "samples".into(),
        filter: serde_json::json!({}),
        time_bucket_secs: 3600,
        group_by: vec!["building".into()],
        aggregations: vec![AggregationSpec {
            func: "sum".into(),
            col: "value_num".into(),
            alias: "kwh".into(),
        }],
        created_by: created_by.into(),
        ext_manifest_hash: None,
    }
}

async fn boot() -> (
    starter_store_postgres::pool::Pool,
    starter_store_postgres::testing::ContainerGuard,
    starter_store_warehouse::ChClient,
    starter_store_warehouse::testing::ContainerGuard,
    Arc<WarehouseRuntime>,
) {
    let (pg, pg_guard) = with_database().await;
    // Apply dimensions migrations against the brand-new pool.
    starter_store_postgres::migrate(&pg)
        .with_source(starter_store_postgres::dimensions::DIMENSIONS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply dimensions migrations");
    let (ch, ch_guard) = with_clickhouse().await;
    let cfg = ch.config().clone();
    let rt = WarehouseRuntime::new(pg.clone(), cfg, WarehouseConfig::default());
    (pg, pg_guard, ch, ch_guard, Arc::new(rt))
}

#[tokio::test]
#[ignore]
async fn w5_mart_define_idempotent_on_identical_hash() {
    let (_pg, _pgg, _ch, _chg, rt) = boot().await;
    let spec = sample_spec("mart_smoke_a", "user:alice");
    let r1 = rt.mart_define(spec.clone()).await.unwrap();
    let r2 = rt.mart_define(spec).await.unwrap();
    assert!(!r1.idempotent_noop);
    assert!(r2.idempotent_noop);
}

#[tokio::test]
#[ignore]
async fn w14_mart_read_rejects_unsupported_filter_keys() {
    let (_pg, _pgg, _ch, _chg, rt) = boot().await;
    let spec = sample_spec("mart_smoke_b", "user:alice");
    rt.mart_define(spec).await.unwrap();
    let q = TagQuery::from_str("floor:\"1\"").unwrap();
    let res = rt
        .mart_read(
            "mart_smoke_b",
            q,
            Utc::now() - Duration::hours(1),
            Utc::now(),
            false,
            20_000,
        )
        .await;
    match res {
        Err(RuntimeError::MartFilterUnsupportedKeys {
            mart,
            unsupported,
            promoted,
        }) => {
            assert_eq!(mart, "mart_smoke_b");
            assert_eq!(unsupported, vec!["floor"]);
            assert!(promoted.contains(&"building".to_string()));
        }
        other => panic!("expected MartFilterUnsupportedKeys, got {other:?}"),
    }
}

#[tokio::test]
#[ignore]
async fn w12_ext_manifest_change_requarantines_live_marts() {
    let (pg, _pgg, _ch, _chg, rt) = boot().await;
    // Seed an approval row at hash A.
    let mut conn = pg.sqlx().acquire().await.unwrap();
    starter_warehouse::catalog::ext::record_approval(
        &mut conn,
        "com.acme.cleaner",
        "hashA",
        "install:initial",
    )
    .await
    .unwrap();
    drop(conn);
    let mut spec = sample_spec("mart_ext_one", "ext:com.acme.cleaner");
    spec.ext_manifest_hash = Some("hashA".into());
    rt.mart_define(spec).await.unwrap();
    // Define a second mart at hashB without prior approval — this
    // re-quarantines mart_ext_one in the same txn.
    let mut spec2 = sample_spec("mart_ext_two", "ext:com.acme.cleaner");
    spec2.ext_manifest_hash = Some("hashB".into());
    let _ = rt.mart_define(spec2).await;
    let row: (String,) = sqlx::query_as("SELECT status FROM marts WHERE name = 'mart_ext_one'")
        .fetch_one(pg.sqlx())
        .await
        .unwrap();
    assert_eq!(row.0, "quarantined");
}

#[tokio::test]
#[ignore]
async fn rf4_sandbox_redefine_refused_when_frozen() {
    let (pg, _pgg, _ch, _chg, rt) = boot().await;
    use starter_warehouse::ddl::sandbox::{SandboxColumn, SandboxSpec};
    let s = SandboxSpec {
        name: "sandbox_test".into(),
        ttl_days: 30,
        columns: vec![SandboxColumn {
            name: "v".into(),
            r#type: "Float64".into(),
        }],
    };
    rt.sandbox_define("user:alice", s.clone(), serde_json::to_value(&s).unwrap())
        .await
        .unwrap();
    // Simulate cleaner.define freezing the sandbox.
    starter_store_postgres::dimensions::sandboxes::freeze(&pg, "sandbox_test", "cleaner_x")
        .await
        .unwrap();
    let r = rt
        .sandbox_redefine("sandbox_test", true, serde_json::json!({"columns": []}))
        .await;
    assert!(matches!(r, Err(RuntimeError::SandboxFrozen { .. })));
}

#[tokio::test]
#[ignore]
async fn rf6_sync_backfill_auto_promotes_beyond_threshold() {
    let (_pg, _pgg, _ch, _chg, rt) = boot().await;
    use starter_warehouse::ddl::cleaner::CleanerSpec;
    // Lower the threshold for the test would require a custom
    // WarehouseConfig; with the default 1_000_000 we pass a
    // larger source_row_count to force the auto-promotion.
    let spec = CleanerSpec {
        name: "cleaner_rf6".into(),
        source_table: "raw_events".into(),
        target_table: "samples".into(),
        projection: "entity_id, ts, value_num".into(),
        backfill: "sync".into(),
        deterministic_key: true,
    };
    let res = rt
        .cleaner_define(spec, "user:alice", 5_000_000)
        .await
        .unwrap();
    assert!(res.auto_promoted, "RF-6 must auto-promote sync→async");
    assert_eq!(res.effective_backfill, "async");
}

#[tokio::test]
#[ignore]
async fn w11_dimension_freshness_envelope_populated() {
    let (_pg, _pgg, _ch, _chg, rt) = boot().await;
    let f = rt.freshness().await.unwrap();
    assert_eq!(f.entities_dict.name, "entities_dict");
}
