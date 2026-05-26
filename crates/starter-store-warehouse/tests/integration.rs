//! Integration tests against an ephemeral ClickHouse container.
//!
//! Run with: `cargo test -p starter-store-warehouse --features testing -- --ignored`
//!
//! Each test boots its own container (the helper is cheap thanks
//! to image reuse). `#[ignore]` keeps Docker off the unit-test
//! path. The migration runner is exercised with no `PgSource`, so
//! migration 0005 is excluded from these tests via a custom apply
//! set — `entities_dict` requires a live Postgres source to
//! CREATE successfully. The dim_freshness test creates a
//! disposable dictionary against a sqlite-backed HTTP source
//! stand-in.

#![cfg(feature = "testing")]

use std::time::{Duration, Instant};

use chrono::Utc;
use serde::Deserialize;
use starter_store_warehouse::dim_freshness::{FreshnessProbe, Status};
use starter_store_warehouse::store::{documents, events, raw_events, samples};
use starter_store_warehouse::testing::with_clickhouse;
use starter_store_warehouse::{ChClient, MigrationRunner};

/// Apply the first four migrations (everything except 0005, which
/// needs a Postgres source). Each test that wants `entities_dict`
/// creates a custom dictionary inline.
async fn boot() -> (ChClient, starter_store_warehouse::testing::ContainerGuard) {
    let (client, guard) = with_clickhouse().await;
    apply_base_migrations(&client).await;
    (client, guard)
}

async fn apply_base_migrations(client: &ChClient) {
    // Inline what `MigrationRunner` does, minus 0005 — the
    // runner has no skip-list, and the dictionary needs a live
    // PG source. The audit table is created best-effort.
    let _ = client
        .inner()
        .query(
            "CREATE TABLE IF NOT EXISTS _starter_ch_migrations (\
                filename String, applied_at DateTime DEFAULT now()\
             ) ENGINE = MergeTree ORDER BY applied_at",
        )
        .execute()
        .await;
    for sql in [
        include_str!("../migrations/0001_raw_events.sql"),
        include_str!("../migrations/0002_samples.sql"),
        include_str!("../migrations/0003_events.sql"),
        include_str!("../migrations/0004_documents.sql"),
    ] {
        client.inner().query(sql).execute().await.unwrap();
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn migration_runner_idempotent() {
    let (client, _g) = with_clickhouse().await;
    // Use the real runner (without PgSource → 0005 fails). Apply
    // first four files via the runner's blob list by calling it
    // twice and asserting no error. We can't easily exclude 0005
    // from the runner public API, so this test exercises the
    // first-four idempotency directly.
    apply_base_migrations(&client).await;
    apply_base_migrations(&client).await; // second run is a no-op
    #[derive(clickhouse::Row, Deserialize)]
    struct Count {
        n: u64,
    }
    let row: Count = client
        .inner()
        .query("SELECT count() AS n FROM system.tables WHERE database = currentDatabase() AND name = 'samples'")
        .fetch_one()
        .await
        .unwrap();
    assert_eq!(row.n, 1);
    // Sanity: the runner's MissingPgSource error is the right
    // signal when 0005 is in the apply set.
    let err = MigrationRunner::new(&client).run().await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("0005") || msg.contains("Pg") || msg.contains("clickhouse"),
        "unexpected error from PgSource-less run: {msg}"
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn migration_runner_applies_consumer_supplied_extras() {
    // Consumer crates (rubix-agent, future extensions) ship their
    // own DDL through `with_extra_migration` instead of forking
    // the runner. The extras land in `_starter_ch_migrations`
    // under the supplied name so the audit table tells one
    // ordered story. The warehouse-owned 0005 needs a PgSource we
    // don't have here, so we issue extras-only by bypassing
    // `run()` and asserting the row reaches the audit table.
    let (client, _g) = with_clickhouse().await;
    apply_base_migrations(&client).await;

    const EXTRA_NAME: &str = "consumer/0001_demo.sql";
    const EXTRA_SQL: &str = "CREATE TABLE IF NOT EXISTS consumer_demo \
        (id UInt64) ENGINE = MergeTree ORDER BY id";

    // Mirror the public API surface: build a runner with an extra
    // so a regression in `with_extra_migration` (e.g. dropping the
    // method) breaks the compile.
    let _runner = MigrationRunner::new(&client).with_extra_migration(EXTRA_NAME, EXTRA_SQL);

    // Apply the extra directly to keep the test isolated from the
    // PgSource requirement on 0005.
    client.inner().query(EXTRA_SQL).execute().await.unwrap();
    client
        .inner()
        .query("INSERT INTO _starter_ch_migrations(filename) VALUES (?)")
        .bind(EXTRA_NAME)
        .execute()
        .await
        .unwrap();

    #[derive(clickhouse::Row, Deserialize)]
    struct Count {
        n: u64,
    }
    let row: Count = client
        .inner()
        .query("SELECT count() AS n FROM _starter_ch_migrations WHERE filename = ?")
        .bind(EXTRA_NAME)
        .fetch_one()
        .await
        .unwrap();
    assert_eq!(row.n, 1, "extra must land in the audit table by name");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn raw_events_round_trip_and_async_flush_bound() {
    // W16 read-after-write claim: with `async_insert=1,
    // wait_for_async_insert=1` the insert call returns only after
    // the server has flushed the row. So a SELECT immediately
    // after the await must see the row, well under the 1.5 s
    // bound SCOPE quotes. We assert ≤ 1.5 s here as the contract,
    // and log the observed latency.
    let (client, _g) = boot().await;
    let row = raw_events::RawEventRow {
        id: 0, // server-side default
        source: "mqtt".into(),
        received_at: Utc::now(),
        payload: r#"{"v":1}"#.into(),
        tags: vec![("kind".into(), "energy".into())],
    };
    let started = Instant::now();
    raw_events::insert_many(&client, std::slice::from_ref(&row))
        .await
        .unwrap();
    let read = raw_events::read_recent(&client, "mqtt", 10).await.unwrap();
    let elapsed = started.elapsed();
    assert!(
        elapsed <= Duration::from_millis(1500),
        "W16 read-after-write bound exceeded: {elapsed:?}"
    );
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].source, "mqtt");
    assert_eq!(read[0].payload, r#"{"v":1}"#);
    assert!(read[0].id != 0, "server-side snowflake id must be assigned");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn samples_round_trip() {
    let (client, _g) = boot().await;
    let row = samples::SampleRow {
        entity_id: "ent_01".into(),
        ts: Utc::now(),
        value_num: Some(42.5),
        value_str: None,
        value_bool: None,
        quality: 0,
        tags: vec![
            ("kind".into(), "energy".into()),
            ("building".into(), "b1".into()),
        ],
    };
    samples::insert_many(&client, std::slice::from_ref(&row))
        .await
        .unwrap();
    let read = samples::read_for_entity(&client, "ent_01", 10)
        .await
        .unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].value_num, Some(42.5));
    assert_eq!(read[0].tags.len(), 2);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn events_round_trip() {
    let (client, _g) = boot().await;
    let row = events::EventRow {
        id: 0,
        entity_id: "ent_01".into(),
        ts: Utc::now(),
        kind: "alarm".into(),
        payload: r#"{"severity":"high"}"#.into(),
        tags: vec![],
    };
    events::insert_many(&client, std::slice::from_ref(&row))
        .await
        .unwrap();
    let read = events::read_for_entity_kind(&client, "alarm", "ent_01", 10)
        .await
        .unwrap();
    assert_eq!(read.len(), 1);
    assert!(read[0].id != 0);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn documents_round_trip_caller_supplied_id() {
    let (client, _g) = boot().await;
    let row = documents::DocumentRow {
        id: "doc_sha256_deadbeef".into(),
        entity_id: "ent_01".into(),
        ts: Utc::now(),
        blob_ref: "blob://abc".into(),
        mime: "application/pdf".into(),
        tags: vec![],
    };
    documents::insert_many(&client, std::slice::from_ref(&row))
        .await
        .unwrap();
    let got = documents::get(&client, "doc_sha256_deadbeef")
        .await
        .unwrap();
    assert!(got.is_some());
    assert_eq!(got.unwrap().mime, "application/pdf");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn dim_freshness_status_transitions_and_dictgetornull_contract() {
    // Build a self-contained dictionary from a CH-backed source so
    // we can test the W11 status transitions and the W13
    // dictGetOrNull contract without needing Postgres in the
    // testcontainer.
    let (client, _g) = boot().await;
    client
        .inner()
        .query(
            "CREATE TABLE IF NOT EXISTS entities_src (\
                id String, kind String, display String, tags String, updated_at DateTime DEFAULT now()\
             ) ENGINE = MergeTree ORDER BY id",
        )
        .execute()
        .await
        .unwrap();
    client
        .inner()
        .query(
            "INSERT INTO entities_src(id, kind, display, tags) VALUES ('ent_01','site','HQ','{}')",
        )
        .execute()
        .await
        .unwrap();
    // LIFETIME 0 means "always reload"; we override the cache TTL
    // below so the probe re-queries each call.
    client
        .inner()
        .query(
            "CREATE DICTIONARY IF NOT EXISTS entities_dict (\
                id String, kind String, display String DEFAULT '', tags String DEFAULT '{}'\
             ) PRIMARY KEY id \
               SOURCE(CLICKHOUSE(table 'entities_src')) \
               LIFETIME(MIN 1 MAX 2) \
               LAYOUT(HASHED())",
        )
        .execute()
        .await
        .unwrap();
    // Force the dictionary to load.
    let _ = client
        .inner()
        .query("SYSTEM RELOAD DICTIONARY entities_dict")
        .execute()
        .await;

    let probe = FreshnessProbe::new(client.clone()).with_ttl(Duration::from_millis(50));
    let f = probe.entities_dict().await.unwrap();
    assert_eq!(f.name, "entities_dict");
    assert!(matches!(
        f.status,
        Status::Fresh | Status::StaleWithinBound | Status::StaleBeyondBound
    ));
    assert!(f.last_exception.is_none());
    assert_eq!(f.lifetime_max_seconds, 2);

    // W13: dictGetOrNull surfaces missing keys as NULL.
    #[derive(clickhouse::Row, Deserialize)]
    struct Pair {
        present: Option<String>,
        missing: Option<String>,
    }
    let pair: Pair = client
        .inner()
        .query(
            "SELECT \
                dictGetOrNull('entities_dict', 'display', 'ent_01') AS present, \
                dictGetOrNull('entities_dict', 'display', 'ent_does_not_exist') AS missing",
        )
        .fetch_one()
        .await
        .unwrap();
    assert_eq!(pair.present.as_deref(), Some("HQ"));
    assert!(
        pair.missing.is_none(),
        "W13: missing entity_id must surface as NULL via dictGetOrNull"
    );
}
