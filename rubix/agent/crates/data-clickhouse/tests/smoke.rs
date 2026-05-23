//! Phase-0 smoke. Boots an ephemeral ClickHouse testcontainer via
//! the starter testing seam and runs `SELECT 1` to prove the seam
//! composes cleanly into rubix even before any schema lands.
//!
//! Marked `#[ignore]` per the starter convention for tests that
//! require Docker. Run with:
//!
//! ```text
//! cargo test -p rubix-data-clickhouse -- --ignored
//! ```
//!
//! Prerequisites for the Docker side are documented in
//! `rubix/docs/testing/SETUP.md`.

use serde::Deserialize;
use starter_store_clickhouse::testing::with_clickhouse;

#[derive(clickhouse::Row, Deserialize)]
struct OneRow {
    one: u8,
}

#[tokio::test]
#[ignore = "requires docker"]
async fn testing_seam_composes_select_one() {
    let (client, _guard) = with_clickhouse().await;

    let row: OneRow = client
        .inner()
        .query("SELECT 1 AS one")
        .fetch_one()
        .await
        .expect("SELECT 1 against testcontainer clickhouse");

    assert_eq!(
        row.one, 1,
        "starter testing seam returned an unexpected value"
    );
}
