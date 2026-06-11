//! RW-07b acceptance: a process-runtime-style `ingest.write` push lands rows in
//! a datasource sink, with the tenant stamped by the host (never the payload).
//!
//! Drives the real host-method domain ([`nexus_api::extensions::ingest::write`])
//! against a live `http_ingest → json_to_arrow → datasource` flow: the push
//! carries a row that *lies* about its tenant, and the assertion proves the
//! landed row carries the caller's tenant instead — the spec's "tenant stamped
//! by host, verified" bullet.

#![cfg(feature = "testing")]

use std::time::Duration;

use nexus_api::extensions::ingest;
use nexus_store::datasource::{self, Envelope, NewDatasource};
use nexus_store::testing::runtime_pool;
use serde_json::{json, Value};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::ingest::IngestWriteResponse;
use starter_store_postgres::testing::with_database;

fn envelope() -> Envelope {
    Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap()
}

fn self_referencing(host: &str, port: i32) -> NewDatasource {
    NewDatasource {
        name: "ingest-target".into(),
        kind: "postgres".into(),
        host: host.into(),
        port,
        database: "postgres".into(),
        db_user: "postgres".into(),
        secret: Some("postgres".into()),
        config: None,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn ingest_write_lands_rows_with_host_stamped_tenant() {
    let (admin, _guard) = with_database().await;
    let opts = admin.sqlx().connect_options();
    let host = opts.get_host().to_string();
    let port = opts.get_port() as i32;
    let pg = runtime_pool(admin.sqlx()).await;
    let env = envelope();

    // The sink table carries a tenant_id column so we can prove the stamp.
    sqlx::query("CREATE TABLE ext_readings (device text, tenant_id text)")
        .execute(admin.sqlx())
        .await
        .expect("create table");

    let created = datasource::insert(&pg, &env, "acme", &self_referencing(&host, port))
        .await
        .expect("register datasource");
    let resolved = datasource::resolve_sink_config(
        &pg, &env, "acme", "tester", created.id, "ext_readings", Some(1), None,
    )
    .await
    .expect("resolve datasource sink config");

    // A push flow: extension pushes → bounded channel → typed → datasource sink.
    let flows = nexus_engine::FlowManager::new().expect("flow manager");
    flows
        .start(
            "ext-ingest-e2e",
            json!({ "type": "http_ingest", "capacity": 64 }),
            vec![
                json!({ "type": "json_to_arrow" }),
                json!({ "type": "sql", "query": "SELECT device, tenant_id FROM flow" }),
            ],
            resolved,
        )
        .expect("start push flow");
    tokio::time::sleep(Duration::from_millis(30)).await;

    // The caller's tenant is `t-real`; the pushed row lies (`tenant_id: evil`).
    let caller = CallerIdentity {
        tenant_id: Some("t-real".into()),
        ..Default::default()
    };
    let resp: IngestWriteResponse = serde_json::from_value(
        ingest::write(
            flows.ingest(),
            json!({
                "source": "ext-ingest-e2e",
                "rows": [{ "device": "a", "tenant_id": "evil" }]
            }),
            Some(&caller),
        )
        .expect("push accepted"),
    )
    .unwrap();
    assert_eq!(resp.accepted, 1);
    assert!(resp.retry_after_secs.is_none());

    // Let the batch drain through the sink (batch_rows = 1 flushes immediately).
    tokio::time::sleep(Duration::from_millis(300)).await;

    let landed: Value = {
        let tenant: String =
            sqlx::query_scalar("SELECT tenant_id FROM ext_readings WHERE device = 'a'")
                .fetch_one(admin.sqlx())
                .await
                .expect("row landed");
        json!(tenant)
    };
    assert_eq!(
        landed, "t-real",
        "the host stamps the caller's tenant, ignoring the payload's"
    );

    flows.stop("ext-ingest-e2e");
}
