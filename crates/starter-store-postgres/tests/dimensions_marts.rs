//! Marts catalog: insert, status transitions, and the W12
//! live-quota trigger.

#![cfg(all(feature = "dimensions", feature = "testing"))]

use sqlx::postgres::types::PgInterval;
use starter_store_postgres::dimensions::{
    marts::{self, InsertMart, MartStatus},
    DIMENSIONS_MIGRATION_SOURCE,
};
use starter_store_postgres::{migrate, testing::with_database, testing::ContainerGuard, Pool};

async fn boot() -> (Pool, ContainerGuard) {
    let (pool, guard) = with_database().await;
    migrate(&pool)
        .with_source(DIMENSIONS_MIGRATION_SOURCE)
        .run()
        .await
        .unwrap();
    (pool, guard)
}

fn one_hour() -> PgInterval {
    PgInterval {
        months: 0,
        days: 0,
        microseconds: 3_600 * 1_000_000,
    }
}

fn insert_mart(name: &str, status: MartStatus) -> InsertMart<'_> {
    InsertMart {
        name,
        description: None,
        source_table: "samples",
        filter: Box::leak(Box::new(serde_json::json!({"tag": {"kind": "energy"}}))),
        time_bucket: one_hour(),
        group_by: Box::leak(Box::new(vec!["building".to_string()])),
        aggregations: Box::leak(Box::new(
            serde_json::json!([{"fn": "sum", "col": "value_num", "as": "kwh"}]),
        )),
        definition_hash: "deadbeef",
        created_by: "user:alice",
        status,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn insert_and_round_trip() {
    let (pool, _g) = boot().await;
    let m = marts::insert(&pool, insert_mart("mart_energy_hourly", MartStatus::Live))
        .await
        .unwrap();
    assert_eq!(m.status, "live");
    assert_eq!(m.group_by, vec!["building".to_string()]);

    // Idempotency: same name with INSERT (no upsert) errors.
    let err = marts::insert(&pool, insert_mart("mart_energy_hourly", MartStatus::Pending)).await;
    assert!(err.is_err());

    marts::set_status(&pool, "mart_energy_hourly", MartStatus::Quarantined)
        .await
        .unwrap();
    let row = marts::get(&pool, "mart_energy_hourly").await.unwrap().unwrap();
    assert_eq!(row.status, "quarantined");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn live_mart_quota_trigger_only_scans_live_rows() {
    let (pool, _g) = boot().await;
    // Lower the quota so the test is cheap.
    sqlx::query("SELECT set_config('warehouse.live_mart_quota', '3', false)")
        .execute(pool.sqlx())
        .await
        .unwrap();

    // 200 non-live rows — these must NOT count against the quota.
    for i in 0..200 {
        let name = format!("mart_q_{i:03}");
        marts::insert(&pool, insert_mart(&name, MartStatus::Quarantined))
            .await
            .unwrap();
    }
    for i in 0..50 {
        let name = format!("mart_f_{i:03}");
        marts::insert(&pool, insert_mart(&name, MartStatus::Failed))
            .await
            .unwrap();
    }

    // Three live rows fit under the quota...
    for i in 0..3 {
        let name = format!("mart_live_{i}");
        marts::insert(&pool, insert_mart(&name, MartStatus::Live))
            .await
            .unwrap();
    }
    assert_eq!(marts::live_count(&pool).await.unwrap(), 3);

    // ...the fourth must trip the trigger.
    let res = marts::insert(&pool, insert_mart("mart_live_3", MartStatus::Live)).await;
    assert!(res.is_err(), "quota trigger must reject the 4th live insert");

    // Quarantining one live mart frees a slot.
    marts::set_status(&pool, "mart_live_0", MartStatus::Quarantined)
        .await
        .unwrap();
    marts::insert(&pool, insert_mart("mart_live_3", MartStatus::Live))
        .await
        .expect("live insert must succeed once a slot frees");

    // A status update that does NOT enter `live` is unaffected by
    // the quota (it skips the count). Promoting an existing non-live
    // row to live with quota already at cap still trips.
    let res = marts::set_status(&pool, "mart_q_001", MartStatus::Live).await;
    assert!(res.is_err());

    // Confirm the partial-index plan is the path used for the live
    // count. We assert the index exists; the planner picks it as
    // the only sensible scan.
    let (idx,): (String,) = sqlx::query_as(
        "SELECT indexdef FROM pg_indexes \
         WHERE tablename = 'marts' AND indexname = 'marts_live_count_idx'",
    )
    .fetch_one(pool.sqlx())
    .await
    .unwrap();
    assert!(idx.contains("WHERE"), "expected partial index: {idx}");
    assert!(idx.contains("'live'"));
}
