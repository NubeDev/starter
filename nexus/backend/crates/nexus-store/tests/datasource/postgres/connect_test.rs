//! The Postgres connector proven against a real container: a datasource sealed
//! at rest is opened into a live pool that authenticates with the recovered
//! secret and runs a guarded query end-to-end. This is the M4 "postgres as a real
//! datasource" path — create → seal → connect → query — not the dev single-pool
//! shortcut.

#![cfg(feature = "testing")]

use std::time::Duration;

use nexus_store::datasource::{self, postgres, Envelope, NewDatasource};
use nexus_store::testing::runtime_pool;
use nexus_store::{run_query, QueryGuards};
use starter_store_postgres::testing::with_database;

fn envelope() -> Envelope {
    Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap()
}

fn guards() -> QueryGuards {
    QueryGuards {
        statement_timeout: Duration::from_secs(5),
        max_rows: 1000,
        max_bytes: 8 * 1024 * 1024,
    }
}

/// A datasource record that points back at the test container itself, with the
/// container's real credentials sealed as the secret. Connecting to it exercises
/// the exact path a user-registered datasource takes.
fn self_referencing(host: &str, port: i32) -> NewDatasource {
    NewDatasource {
        name: "self".into(),
        kind: "postgres".into(),
        host: host.into(),
        port,
        database: "postgres".into(),
        db_user: "postgres".into(),
        secret: "postgres".into(),
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn connect_opens_a_queryable_pool_for_a_sealed_datasource() {
    let (admin, _guard) = with_database().await;

    // Recover the container's host/port from the admin pool's own connect options
    // so the datasource record targets the same database the test is running on.
    let opts = admin.sqlx().connect_options();
    let host = opts.get_host().to_string();
    let port = opts.get_port() as i32;

    // Seed a table on the target DB the datasource will later read.
    sqlx::query("CREATE TABLE readings (id int primary key, watt double precision)")
        .execute(admin.sqlx())
        .await
        .expect("create table");
    sqlx::query("INSERT INTO readings VALUES (1, 240.5), (2, 12.0)")
        .execute(admin.sqlx())
        .await
        .expect("seed");

    let pg = &runtime_pool(admin.sqlx()).await;
    let env = envelope();
    let created = datasource::insert(pg, &env, "acme", &self_referencing(&host, port))
        .await
        .expect("register datasource");

    // Open a pool to the datasource via the audited decrypt boundary…
    let ds_pool = postgres::connect(pg, &env, "acme", "tester", &created)
        .await
        .expect("connect to datasource");

    // …and the pool runs a real query under the R4 guards.
    let out = run_query(&ds_pool, "SELECT id, watt FROM readings ORDER BY id", guards())
        .await
        .expect("query runs");
    assert_eq!(out.stats.row_count, 2);
    assert_eq!(out.rows[1]["watt"], 12.0);

    // A write is still rejected by the read-only guard on the datasource pool.
    let err = run_query(&ds_pool, "INSERT INTO readings VALUES (3, 0)", guards())
        .await
        .expect_err("write rejected");
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("read-only") || msg.contains("read only"), "got: {msg}");
}
