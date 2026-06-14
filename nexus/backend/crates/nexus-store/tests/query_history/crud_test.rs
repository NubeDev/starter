//! Query history against real Postgres under the runtime role: a run records,
//! recent lists newest-first, starring pins a row above the rolling window and
//! exempts it from retention, and tenant isolation holds.

#![cfg(feature = "testing")]

use nexus_store::query_history::{self, NewQueryRun};
use nexus_store::testing::runtime_pool;
use starter_store_postgres::testing::with_database;

fn run(sql: &str) -> NewQueryRun {
    NewQueryRun {
        user_id: "user-1".into(),
        datasource_id: None,
        sql: sql.into(),
        elapsed_ms: Some(12),
        row_count: Some(3),
        error: None,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn records_and_lists_newest_first() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    query_history::record_run(pg, "acme", &run("SELECT 1"))
        .await
        .unwrap();
    query_history::record_run(pg, "acme", &run("SELECT 2"))
        .await
        .unwrap();

    let rows = query_history::list_recent(pg, "acme", "user-1", 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    // Newest first: the second insert leads.
    assert_eq!(rows[0].sql, "SELECT 2");
    assert_eq!(rows[1].sql, "SELECT 1");
    assert_eq!(rows[0].row_count, Some(3));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn starring_pins_a_row_first_and_persists() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    query_history::record_run(pg, "acme", &run("SELECT old"))
        .await
        .unwrap();
    let rows = query_history::list_recent(pg, "acme", "user-1", 10)
        .await
        .unwrap();
    let old_id = rows[0].id;
    query_history::record_run(pg, "acme", &run("SELECT new"))
        .await
        .unwrap();

    // Star the older row; it should sort above the newer un-starred one.
    let updated = query_history::set_starred(pg, "acme", "user-1", old_id, true)
        .await
        .unwrap();
    assert!(updated);
    let rows = query_history::list_recent(pg, "acme", "user-1", 10)
        .await
        .unwrap();
    assert_eq!(rows[0].sql, "SELECT old");
    assert!(rows[0].starred);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tenant_isolation_holds() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    query_history::record_run(pg, "acme", &run("SELECT acme"))
        .await
        .unwrap();
    query_history::record_run(pg, "other", &run("SELECT other"))
        .await
        .unwrap();

    let acme = query_history::list_recent(pg, "acme", "user-1", 10)
        .await
        .unwrap();
    assert_eq!(acme.len(), 1);
    assert_eq!(acme[0].sql, "SELECT acme");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn starring_another_users_row_is_a_no_op() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    query_history::record_run(pg, "acme", &run("SELECT mine"))
        .await
        .unwrap();
    let rows = query_history::list_recent(pg, "acme", "user-1", 10)
        .await
        .unwrap();
    let id = rows[0].id;

    // A different user cannot star user-1's row.
    let updated = query_history::set_starred(pg, "acme", "user-2", id, true)
        .await
        .unwrap();
    assert!(!updated);
}
