//! Integration test for the Phase A.2 `starter_scheduled_flows`
//! migration: confirms the schema applies, the UNIQUE
//! `(tenant_id, flow_id)` constraint is enforced, and the
//! `starter_scheduled_flows` LISTEN/NOTIFY trigger fires on both
//! INSERT and UPDATE-of-`next_run_at` / -`enabled`.
//!
//! Marked `#[ignore]` because it requires Docker. CI runs via
//! `cargo test -p starter-store-postgres --features testing --
//! --ignored`.

#![cfg(feature = "testing")]

use std::time::Duration;

use sqlx::postgres::PgListener;
use starter_store_postgres::{
    migrate, testing::with_database, SCHEDULED_FLOWS_MIGRATION_SOURCE,
};

const CHANNEL: &str = "starter_scheduled_flows";
const SENTINEL_TENANT: &str = "00000000-0000-0000-0000-000000000000";
const SENTINEL_ACTOR: &str = "00000000-0000-0000-0000-000000000000";

#[tokio::test]
#[ignore = "requires docker"]
async fn scheduled_flows_notify_fires_on_insert_and_update() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("scheduled_flows migrations apply");

    // Subscribe BEFORE any inserts so the first NOTIFY is captured.
    let mut listener = PgListener::connect_with(pool.sqlx())
        .await
        .expect("PgListener connect");
    listener.listen(CHANNEL).await.expect("LISTEN");

    // --- INSERT -------------------------------------------------
    let id = "01HXYZSCHEDULE0000000000A1";
    sqlx::query(
        r#"INSERT INTO starter_scheduled_flows
            (id, tenant_id, flow_id, cron_expr, next_run_at, created_by)
           VALUES ($1, $2::uuid, $3, $4, NOW() + INTERVAL '1 hour', $5::uuid)"#,
    )
    .bind(id)
    .bind(SENTINEL_TENANT)
    .bind("com.rubix.weekly-report")
    .bind("0 8 * * 1")
    .bind(SENTINEL_ACTOR)
    .execute(pool.sqlx())
    .await
    .expect("insert row");

    let notif = tokio::time::timeout(Duration::from_secs(10), listener.recv())
        .await
        .expect("insert notify in time")
        .expect("insert notify present");
    assert_eq!(notif.channel(), CHANNEL);
    let payload: serde_json::Value =
        serde_json::from_str(notif.payload()).expect("payload is json");
    assert_eq!(payload["op"], "INSERT");
    assert_eq!(payload["flow_id"], "com.rubix.weekly-report");
    assert_eq!(payload["enabled"], true);

    // --- UPDATE next_run_at ------------------------------------
    sqlx::query(
        r#"UPDATE starter_scheduled_flows
              SET next_run_at = NOW() + INTERVAL '2 hours'
            WHERE id = $1"#,
    )
    .bind(id)
    .execute(pool.sqlx())
    .await
    .expect("update next_run_at");

    let notif = tokio::time::timeout(Duration::from_secs(10), listener.recv())
        .await
        .expect("update notify in time")
        .expect("update notify present");
    let payload: serde_json::Value =
        serde_json::from_str(notif.payload()).expect("payload is json");
    assert_eq!(payload["op"], "UPDATE");
    assert_eq!(payload["enabled"], true);

    // --- UPDATE enabled ----------------------------------------
    sqlx::query("UPDATE starter_scheduled_flows SET enabled = FALSE WHERE id = $1")
        .bind(id)
        .execute(pool.sqlx())
        .await
        .expect("update enabled");

    let notif = tokio::time::timeout(Duration::from_secs(10), listener.recv())
        .await
        .expect("disable notify in time")
        .expect("disable notify present");
    let payload: serde_json::Value =
        serde_json::from_str(notif.payload()).expect("payload is json");
    assert_eq!(payload["op"], "UPDATE");
    assert_eq!(payload["enabled"], false);

    // --- UPDATE of an unrelated column should NOT notify -------
    // (scoped trigger on next_run_at / enabled only).
    sqlx::query(
        r#"UPDATE starter_scheduled_flows
              SET last_run_at = NOW(),
                  last_run_status = 'succeeded'
            WHERE id = $1"#,
    )
    .bind(id)
    .execute(pool.sqlx())
    .await
    .expect("bookkeeping update");

    let quiet = tokio::time::timeout(Duration::from_millis(500), listener.recv()).await;
    assert!(
        quiet.is_err(),
        "bookkeeping-only update must not fire NOTIFY (got {quiet:?})"
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn scheduled_flows_unique_tenant_flow_enforced() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("scheduled_flows migrations apply");

    let insert = |id: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query(
                r#"INSERT INTO starter_scheduled_flows
                    (id, tenant_id, flow_id, cron_expr, next_run_at, created_by)
                   VALUES ($1, $2::uuid, $3, $4, NOW(), $5::uuid)"#,
            )
            .bind(id)
            .bind(SENTINEL_TENANT)
            .bind("com.rubix.weekly-report")
            .bind("0 8 * * 1")
            .bind(SENTINEL_ACTOR)
            .execute(pool.sqlx())
            .await
        }
    };

    insert("01HXYZSCHEDULE0000000000B1").await.expect("first insert");
    let err = insert("01HXYZSCHEDULE0000000000B2")
        .await
        .expect_err("second insert violates UNIQUE(tenant_id, flow_id)");
    let msg = format!("{err}");
    assert!(
        msg.contains("starter_scheduled_flows_unique") || msg.contains("duplicate key"),
        "unexpected error: {msg}"
    );
}
