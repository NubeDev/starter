//! Integration tests for [`PgClipboard`] — mirror of the SQLite
//! tests over an ephemeral testcontainers Postgres.
//!
//! **`#[ignore]`** by default — requires Docker. Run with
//! `cargo test -p starter-clipboard-postgres -- --ignored`.

use std::sync::Arc;

use chrono::{Duration, Utc};
use starter_clipboard::{new_entry, ClipboardStore};
use starter_clipboard_postgres::{migration_source, PgClipboard};
use starter_spi::auth::{Principal, Role};
use starter_spi::Error;
use starter_store_postgres::{migrate, testing::with_database, Pool};

fn principal(subject: &str) -> Principal {
    Principal {
        subject: subject.to_string(),
        role: Role::Writer,
        scopes: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

async fn fresh_pool() -> (Pool, starter_store_postgres::testing::ContainerGuard) {
    let (pool, guard) = with_database().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");
    (pool, guard)
}

#[tokio::test]
#[ignore = "requires docker"]
async fn round_trip_returns_signed_payload() {
    let (pool, _guard) = fresh_pool().await;
    let store = Arc::new(PgClipboard::new(pool, b"unit-test-key-please-rotate").expect("key"));
    let alice = principal("alice");

    let entry = new_entry(
        &alice,
        "note",
        serde_json::json!({"text": "hello", "id": "n1"}),
        Duration::seconds(60),
    )
    .expect("entry");
    let id = entry.id.clone();
    store.put(entry.clone()).await.expect("put");

    let got = store
        .get(&alice.subject, &id)
        .await
        .expect("get")
        .expect("present");
    assert_eq!(got.principal_id, "alice");
    assert_eq!(got.resource_kind, "note");
    assert_eq!(got.payload, entry.payload);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn cross_principal_lookups_return_none() {
    let (pool, _guard) = fresh_pool().await;
    let store = Arc::new(PgClipboard::new(pool, b"unit-test-key-please-rotate").expect("key"));
    let alice = principal("alice");
    let bob = principal("bob");

    let entry = new_entry(
        &alice,
        "note",
        serde_json::json!({"x": 1}),
        Duration::seconds(60),
    )
    .expect("entry");
    let id = entry.id.clone();
    store.put(entry).await.expect("put");

    let bob_lookup = store.get(&bob.subject, &id).await.expect("get");
    assert!(
        bob_lookup.is_none(),
        "bob must not learn alice's entry exists"
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn expired_entries_return_none() {
    let (pool, _guard) = fresh_pool().await;
    let store = Arc::new(PgClipboard::new(pool, b"unit-test-key-please-rotate").expect("key"));
    let alice = principal("alice");

    let mut entry = new_entry(
        &alice,
        "note",
        serde_json::json!({"x": 1}),
        Duration::seconds(60),
    )
    .expect("entry");
    entry.expires_at = Utc::now() - Duration::seconds(1);
    let id = entry.id.clone();
    store.put(entry).await.expect("put");

    assert!(store.get(&alice.subject, &id).await.expect("get").is_none());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tampered_payload_fails_closed() {
    let (pool, _guard) = fresh_pool().await;
    let store = PgClipboard::new(pool.clone(), b"unit-test-key-please-rotate").expect("key");
    let alice = principal("alice");

    let entry = new_entry(
        &alice,
        "note",
        serde_json::json!({"text": "ok"}),
        Duration::seconds(60),
    )
    .expect("entry");
    let id = entry.id.clone();
    store.put(entry).await.expect("put");

    sqlx::query(r#"UPDATE starter_clipboard SET payload = $1 WHERE id = $2"#)
        .bind(r#"{"text": "tampered"}"#)
        .bind(&id)
        .execute(pool.sqlx())
        .await
        .expect("update");

    let err = store
        .get(&alice.subject, &id)
        .await
        .expect_err("must reject tampered row");
    assert!(matches!(err, Error::Invalid { .. }), "got {err:?}");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn rotated_key_invalidates_existing_entries() {
    let (pool, _guard) = fresh_pool().await;
    let alice = principal("alice");

    let entry = new_entry(
        &alice,
        "note",
        serde_json::json!({"x": 1}),
        Duration::seconds(60),
    )
    .expect("entry");
    let id = entry.id.clone();

    let before = PgClipboard::new(pool.clone(), b"old-key").expect("key");
    before.put(entry).await.expect("put");

    let after = PgClipboard::new(pool, b"new-key").expect("key");
    let err = after
        .get(&alice.subject, &id)
        .await
        .expect_err("must reject row signed under old key");
    assert!(matches!(err, Error::Invalid { .. }));
}
