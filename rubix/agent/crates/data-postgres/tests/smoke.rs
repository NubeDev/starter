//! Phase-0 smoke. Boots an ephemeral Postgres testcontainer via
//! the starter testing seam and runs `SELECT 1` to prove the seam
//! composes cleanly into rubix even before any schema lands.
//!
//! Marked `#[ignore]` per the starter convention for tests that
//! require Docker. Run with:
//!
//! ```text
//! cargo test -p rubix-data-postgres -- --ignored
//! ```
//!
//! Prerequisites for the Docker side are documented in
//! `rubix/docs/testing/SETUP.md`.

use sqlx::Row;
use starter_store_postgres::testing::with_database;

#[tokio::test]
#[ignore = "requires docker"]
async fn testing_seam_composes_select_one() {
    let (pool, _guard) = with_database().await;

    let row = sqlx::query("SELECT 1::int4 AS one")
        .fetch_one(pool.sqlx())
        .await
        .expect("SELECT 1 against testcontainer postgres");

    let one: i32 = row.get("one");
    assert_eq!(one, 1, "starter testing seam returned an unexpected value");
}
