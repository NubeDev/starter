//! End-to-end TimescaleDB smoke test (Stage 2 of
//! warehouse-engine-swap): ingest → mart cagg → query →
//! retention drop, all driven through the `tsdb` surface plus
//! the `TimescaleDbDialect` cagg DDL.
//!
//! `#[ignore]` by default because it requires Docker. Run with:
//!
//! ```text
//! cargo test -p starter-warehouse --features warehouse,testing \
//!     --test tsdb_smoke -- --ignored
//! ```

#![cfg(all(feature = "warehouse", feature = "testing"))]

use chrono::{Duration, Utc};
use sqlx::Row;
use starter_store_warehouse::tsdb::{
    cagg, retention, run_migrations,
    store::samples::{self, SampleRow},
    testing::with_timescale,
};
use starter_warehouse::catalog::mart_spec::{AggregationSpec, MartSpec};
use starter_warehouse::ddl::{mart, DdlDialect, TimescaleDbDialect};

#[tokio::test]
#[ignore = "requires docker"]
async fn ingest_cagg_query_retention_round_trip() {
    let (client, _guard) = with_timescale().await;
    run_migrations(&client).await.expect("apply migrations");

    // Pre-seed the entities dimension so the direct JOIN in the
    // mart read query (replacing the gone `entities_dict` dict)
    // has a row to match against.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS entities (\
         id TEXT NOT NULL, tenant_id TEXT NOT NULL, display TEXT, \
         PRIMARY KEY (id, tenant_id))",
    )
    .execute(client.pool())
    .await
    .unwrap();
    sqlx::query("INSERT INTO entities (id, tenant_id, display) VALUES ($1, $2, $3)")
        .bind("sensor-1")
        .bind("acme")
        .bind("Sensor One")
        .execute(client.pool())
        .await
        .unwrap();

    // L2 ingest via COPY.
    let now = Utc::now();
    let mut rows = Vec::new();
    for i in 0..120 {
        rows.push(SampleRow {
            tenant_id: "acme".into(),
            entity_id: "sensor-1".into(),
            ts: now - Duration::seconds(i),
            value_num: Some(i as f64),
            value_str: None,
            value_bool: None,
            quality: 0,
            tags: serde_json::json!({"kind": "energy"}),
        });
    }
    samples::insert_many(&client, &rows)
        .await
        .expect("copy samples");
    let count = samples::count_for_entity(
        &client,
        "sensor-1",
        now - Duration::seconds(300),
        now + Duration::seconds(60),
    )
    .await
    .unwrap();
    assert_eq!(count, 120);

    // Build a continuous aggregate via the dialect.
    let spec = MartSpec {
        name: "mart_smoke".into(),
        description: None,
        source_table: "samples".into(),
        filter: serde_json::json!({}),
        time_bucket_secs: 60,
        group_by: vec!["entity_id".into()],
        aggregations: vec![AggregationSpec {
            func: "sum".into(),
            col: "value_num".into(),
            alias: "value_sum".into(),
        }],
        created_by: "user:smoke".into(),
        ext_manifest_hash: None,
    };
    let ddl = TimescaleDbDialect.mart_create_ddl(&spec).unwrap();
    sqlx::query(&ddl.create_view)
        .execute(client.pool())
        .await
        .unwrap();
    sqlx::query(&ddl.create_target)
        .execute(client.pool())
        .await
        .unwrap();

    // Force a refresh so the inserted rows materialize into the
    // cagg without waiting for the scheduled policy.
    cagg::refresh(
        &client,
        "mart_smoke",
        now - Duration::seconds(600),
        now + Duration::seconds(120),
    )
    .await
    .expect("refresh cagg");

    // Snapshot read path (replaces `SHOW CREATE TABLE`).
    let snap = cagg::view_snapshot(&client, "mart_smoke")
        .await
        .unwrap()
        .expect("snapshot row");
    assert!(snap.view_definition.contains("time_bucket"));

    // Read via the cagg with the direct-JOIN query.
    let q = mart::read_query_pg(&spec, "", false);
    let rows = sqlx::query(&q)
        .bind(now - Duration::seconds(600))
        .bind(now + Duration::seconds(120))
        .fetch_all(client.pool())
        .await
        .expect("query mart");
    assert!(!rows.is_empty(), "cagg should have materialized rows");
    let display: Option<String> = rows[0].try_get("display_for_entity_id").unwrap();
    assert_eq!(display.as_deref(), Some("Sensor One"));

    // Retention round-trip on the raw L1 table.
    retention::add_retention_policy(&client, "raw_events", 7)
        .await
        .expect("add retention");
    let days = retention::snapshot_days(&client, "raw_events")
        .await
        .unwrap();
    assert_eq!(days, Some(7));
    retention::remove_retention_policy(&client, "raw_events")
        .await
        .expect("remove retention");
    let days = retention::snapshot_days(&client, "raw_events")
        .await
        .unwrap();
    assert_eq!(days, None);
}
