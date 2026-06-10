//! Datasource sink acceptance: a flow `memory → json_to_arrow → datasource`
//! lands rows in a Postgres datasource via the tenant's datasource record — the
//! RW-04 write path. The datasource points back at the test container, so the run
//! exercises register → seal → resolve (audited decrypt) → connect → COPY write,
//! exactly as a user's flow would.

#![cfg(feature = "testing")]

use std::time::Duration;

use nexus_store::datasource::{self, Envelope, NewDatasource};
use nexus_store::testing::runtime_pool;
use serde_json::json;
use starter_store_postgres::testing::with_database;

fn envelope() -> Envelope {
    Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap()
}

/// A datasource record pointing back at the test container itself, credentials
/// sealed, so writing to it exercises the real resolve+connect path.
fn self_referencing(host: &str, port: i32) -> NewDatasource {
    NewDatasource {
        name: "sink-target".into(),
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
async fn flow_writes_rows_to_a_datasource_via_copy() {
    let (admin, _guard) = with_database().await;
    let opts = admin.sqlx().connect_options();
    let host = opts.get_host().to_string();
    let port = opts.get_port() as i32;
    let pg = runtime_pool(admin.sqlx()).await;
    let env = envelope();

    // The destination table the sink COPYs into.
    sqlx::query("CREATE TABLE device_readings (device text, watt double precision)")
        .execute(admin.sqlx())
        .await
        .expect("create table");

    // Register the datasource (sealed secret) the flow output references by id.
    let created = datasource::insert(&pg, &env, "acme", &self_referencing(&host, port))
        .await
        .expect("register datasource");

    // Resolve the `{type:datasource, datasource:id, table}` output to the engine's
    // connection material — the audited decrypt boundary the start handler uses.
    let resolved = datasource::resolve_sink_config(
        &pg,
        &env,
        "acme",
        "tester",
        created.id,
        "device_readings",
        Some(10),
        None,
    )
    .await
    .expect("resolve datasource sink config");

    // Drive the full pipeline: a finite memory source of JSON docs, typed by
    // json_to_arrow, written by the resolved datasource sink. FlowManager runs it
    // as a background task; the memory source is finite so the run completes.
    let flows = nexus_engine::FlowManager::new().expect("flow manager");
    flows
        .start(
            "rw04-e2e",
            json!({
                "type": "memory",
                "messages": [
                    r#"{"device":"a","watt":240.5}"#,
                    r#"{"device":"b","watt":12.0}"#
                ]
            }),
            vec![json!({ "type": "json_to_arrow" })],
            resolved,
        )
        .expect("start flow");

    // Wait for the finite run to drain and close (flush on close writes the tail).
    for _ in 0..100 {
        if !flows.is_running("rw04-e2e") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(!flows.is_running("rw04-e2e"), "the finite flow should finish");
    // A small grace for the close()/COPY to commit after the task drops from the set.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM device_readings")
        .fetch_one(admin.sqlx())
        .await
        .expect("count");
    assert_eq!(rows, 2, "both rows landed via the datasource sink");

    let watt: f64 =
        sqlx::query_scalar("SELECT watt FROM device_readings WHERE device = 'b'")
            .fetch_one(admin.sqlx())
            .await
            .expect("value");
    assert_eq!(watt, 12.0);
}
