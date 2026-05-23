//! Integration tests for [`SqliteClipboard`].
//!
//! Covers the four guarantees from SCOPE §"Storage shape" /
//! §"Security & privacy":
//!
//! 1. Round-trip (`put` then `get` returns the same entry).
//! 2. Cross-principal lookups return `None` (no existence leak).
//! 3. Expired entries return `None`.
//! 4. A tampered row (signature mismatch) fails closed with
//!    [`starter_spi::Error::Invalid`].

use std::sync::Arc;

use chrono::{Duration, Utc};
use starter_clipboard::{new_entry, ClipboardStore};
use starter_clipboard_sqlite::{migration_source, SqliteClipboard};
use starter_spi::auth::{Principal, Role};
use starter_spi::Error;
use starter_store_sqlite::{migrate, testing::ephemeral};

fn principal(subject: &str) -> Principal {
    Principal {
        subject: subject.to_string(),
        role: Role::Writer,
        scopes: Vec::new(),
        tenant_id: None,
        extra: serde_json::Value::Null,
    }
}

async fn setup() -> Arc<SqliteClipboard> {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");
    Arc::new(SqliteClipboard::new(pool, b"unit-test-key-please-rotate").expect("hmac key ok"))
}

#[tokio::test]
async fn round_trip_returns_signed_payload() {
    let store = setup().await;
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
async fn cross_principal_lookups_return_none() {
    let store = setup().await;
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
async fn expired_entries_return_none() {
    let store = setup().await;
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
async fn tampered_payload_fails_closed() {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");
    let store =
        SqliteClipboard::new(pool.clone(), b"unit-test-key-please-rotate").expect("hmac key ok");
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

    // Mutate the payload directly under the store's feet without
    // re-signing — simulates a row-level write through some other
    // path (e.g. SQL injection, ops-team UPDATE, key rotation
    // applied lazily, etc.).
    sqlx::query(r#"UPDATE starter_clipboard SET payload = ?1 WHERE id = ?2"#)
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
async fn rotated_key_invalidates_existing_entries() {
    // Write with one key, then rebuild the store with a different
    // key and confirm reads fail closed — proving the SCOPE
    // "rotated per-deploy" guarantee.
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");
    let alice = principal("alice");

    let entry = new_entry(
        &alice,
        "note",
        serde_json::json!({"x": 1}),
        Duration::seconds(60),
    )
    .expect("entry");
    let id = entry.id.clone();

    let before = SqliteClipboard::new(pool.clone(), b"old-key").expect("hmac key ok");
    before.put(entry).await.expect("put");

    let after = SqliteClipboard::new(pool, b"new-key").expect("hmac key ok");
    let err = after
        .get(&alice.subject, &id)
        .await
        .expect_err("must reject row signed under old key");
    assert!(matches!(err, Error::Invalid { .. }));
}
