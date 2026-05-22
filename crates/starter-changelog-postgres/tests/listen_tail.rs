//! Integration tests for [`PgListenTail`].
//!
//! Spin up a real Postgres container, install both migrations
//! (table + LISTEN/NOTIFY trigger), subscribe, append a couple of
//! rows, and confirm both are delivered without polling delay.
//!
//! **`#[ignore]`** by default — requires Docker. Run with
//! `cargo test -p starter-changelog-postgres -- --ignored`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use starter_changelog::ChangeTail;
use starter_changelog_postgres::{migration_source, PgChangeRecorder, PgListenTail};
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Actor, Change, ChangeId, ChangeRecorder, ChangeTx, GroupId, Op};
use starter_spi::Result;
use starter_store_postgres::{migrate, testing::with_database};

fn note_change(after: serde_json::Value) -> Change {
    Change {
        id: ChangeId(String::new()),
        at: Utc::now(),
        actor: Actor::User {
            subject: "alice".into(),
        },
        resource: ResourceRef::row("note", "n1"),
        resource_version: Some(1),
        op: Op::Create,
        before: None,
        after: Some(after),
        patch: None,
        group_id: GroupId(String::new()),
        correlation: None,
    }
}

async fn record_one(recorder: &PgChangeRecorder, after: serde_json::Value) -> ChangeId {
    let captured: Arc<Mutex<Option<ChangeId>>> = Arc::new(Mutex::new(None));
    let captured_inner = captured.clone();
    recorder
        .transaction(Box::new(move |tx: &dyn ChangeTx| {
            let captured = captured_inner.clone();
            let ch = note_change(after);
            Box::pin(async move {
                let id = tx.record(ch).await?;
                *captured.lock().unwrap() = Some(id);
                Ok(()) as Result<()>
            })
        }))
        .await
        .expect("recorder tx");
    let id = captured.lock().unwrap().clone().expect("change id");
    id
}

#[tokio::test]
#[ignore = "requires docker"]
async fn listen_tail_delivers_appended_rows() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let recorder = PgChangeRecorder::new(pool.clone());
    let tail = PgListenTail::new(pool.clone()).with_buffer(8);

    // Seed one row BEFORE subscribing so it ends up in the cursor
    // snapshot and is NOT delivered (matches `PgPollingTail`
    // semantics).
    let _seed = record_one(&recorder, serde_json::json!({"text": "seed"})).await;

    let mut rx = tail.subscribe().await.expect("subscribe");

    // Now append two rows; both should arrive via NOTIFY.
    let id1 = record_one(&recorder, serde_json::json!({"text": "one"})).await;
    let id2 = record_one(&recorder, serde_json::json!({"text": "two"})).await;

    let got1 = tokio::time::timeout(Duration::from_secs(10), rx.recv())
        .await
        .expect("first row in time")
        .expect("first row present");
    let got2 = tokio::time::timeout(Duration::from_secs(10), rx.recv())
        .await
        .expect("second row in time")
        .expect("second row present");

    assert_eq!(got1.id, id1);
    assert_eq!(got2.id, id2);
}

/// The notify-driven path is the happy path. Confirm the safety
/// re-poll keeps draining if the listener is woken without a
/// notification (we simulate this by configuring a very short
/// safety interval and inserting a row using the same connection
/// pool).
#[tokio::test]
#[ignore = "requires docker"]
async fn safety_repoll_picks_up_rows_too() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let recorder = PgChangeRecorder::new(pool.clone());
    let tail = PgListenTail::new(pool.clone())
        .with_buffer(4)
        .with_safety_interval(Duration::from_millis(100));

    let mut rx = tail.subscribe().await.expect("subscribe");

    let id = record_one(&recorder, serde_json::json!({"text": "x"})).await;
    let got = tokio::time::timeout(Duration::from_secs(10), rx.recv())
        .await
        .expect("row in time")
        .expect("row present");
    assert_eq!(got.id, id);
}
