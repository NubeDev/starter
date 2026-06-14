//! The M0 query path proven against a real Postgres (testcontainers): user SQL
//! runs under the control-plane guards, returns real rows with a derived column
//! schema, rejects writes via the read-only transaction, and truncates at the
//! row cap instead of streaming an unbounded result.

#![cfg(feature = "testing")]

use std::time::Duration;

use nexus_store::{run_query, QueryGuards};
use starter_store_postgres::testing::with_database;

async fn seed(pool: &sqlx::PgPool) {
    sqlx::query(
        "CREATE TABLE samples (id int primary key, city text, temp_c double precision, ok boolean)",
    )
    .execute(pool)
    .await
    .expect("create table");
    sqlx::query(
        "INSERT INTO samples (id, city, temp_c, ok) VALUES \
         (1,'berlin',21.5,true),(2,'madrid',33.0,false),(3,'oslo',-4.0,true)",
    )
    .execute(pool)
    .await
    .expect("seed rows");
}

fn guards() -> QueryGuards {
    QueryGuards {
        statement_timeout: Duration::from_secs(5),
        max_rows: 1000,
        max_bytes: 8 * 1024 * 1024,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn returns_real_rows_with_schema() {
    let (pool, _guard) = with_database().await;
    seed(pool.sqlx()).await;

    let out = run_query(
        pool.sqlx(),
        "SELECT id, city, temp_c, ok FROM samples ORDER BY id",
        guards(),
    )
    .await
    .expect("query runs");

    assert_eq!(out.stats.row_count, 3);
    assert!(!out.stats.truncated);
    let names: Vec<&str> = out.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["id", "city", "temp_c", "ok"]);
    assert_eq!(out.rows[0]["city"], "berlin");
    assert_eq!(out.rows[1]["temp_c"], 33.0);
    assert_eq!(out.rows[2]["ok"], true);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn predicate_and_limit_push_down_to_postgres() {
    let (pool, _guard) = with_database().await;
    seed(pool.sqlx()).await;

    let out = run_query(
        pool.sqlx(),
        "SELECT city FROM samples WHERE ok = true ORDER BY city",
        guards(),
    )
    .await
    .expect("query runs");

    // WHERE ran in Postgres — only the two ok=true rows come back.
    let cities: Vec<&str> = out
        .rows
        .iter()
        .map(|r| r["city"].as_str().unwrap())
        .collect();
    assert_eq!(cities, ["berlin", "oslo"]);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn read_only_guard_rejects_writes() {
    let (pool, _guard) = with_database().await;
    seed(pool.sqlx()).await;

    let err = run_query(
        pool.sqlx(),
        "INSERT INTO samples (id, city, temp_c, ok) VALUES (9,'x',0,true)",
        guards(),
    )
    .await
    .expect_err("a write must be rejected by the read-only transaction");

    // Rejected by Postgres' read-only transaction, not by string-matching.
    let msg = format!("{err}").to_lowercase();
    assert!(
        msg.contains("read-only") || msg.contains("read only"),
        "got: {msg}"
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn row_cap_truncates() {
    let (pool, _guard) = with_database().await;
    sqlx::query("CREATE TABLE big (n int)")
        .execute(pool.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO big SELECT generate_series(1, 500)")
        .execute(pool.sqlx())
        .await
        .unwrap();

    let mut g = guards();
    g.max_rows = 10;
    let out = run_query(pool.sqlx(), "SELECT n FROM big ORDER BY n", g)
        .await
        .expect("query runs");

    assert!(out.stats.truncated, "hitting the row cap is reported");
    assert_eq!(
        out.stats.row_count, 10,
        "the cap stops the fetch, no unbounded buffer"
    );
}
