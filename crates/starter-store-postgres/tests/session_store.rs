//! Integration tests for [`PgSessionStore`]. Twin of the
//! `session_store_*` cases in
//! `starter-store-sqlite/tests/flow.rs`.

#![cfg(all(feature = "flow", feature = "testing"))]

use starter_flow_spi::flow::{SessionId, SessionRecord, SessionStore};
use starter_flow_spi::Principal;
use starter_spi::auth::Role;
use starter_store_postgres::flow::{PgSessionStore, FLOW_MIGRATION_SOURCE};
use starter_store_postgres::{migrate, testing::with_database, testing::ContainerGuard, Pool};

async fn boot() -> (Pool, ContainerGuard) {
    let (pool, guard) = with_database().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    (pool, guard)
}

fn fresh_principal() -> Principal {
    Principal {
        subject: "u-1".into(),
        role: Role::Reader,
        scopes: vec![],
        tenant_id: None,
        teams: Vec::new(),
        tenant_scope: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn session_store_put_get_list_by_principal() {
    let (pool, _guard) = boot().await;
    let store = PgSessionStore::new(pool);

    let p1 = fresh_principal();
    let mut p2 = fresh_principal();
    p2.subject = "u-2".into();

    let s1 = SessionId::new();
    let s2 = SessionId::new();
    let s3 = SessionId::new();
    store
        .put(
            s1,
            SessionRecord::new(s1, p1.clone(), serde_json::json!({"k": 1})),
        )
        .await
        .unwrap();
    store
        .put(
            s2,
            SessionRecord::new(s2, p1.clone(), serde_json::json!({"k": 2})),
        )
        .await
        .unwrap();
    store
        .put(
            s3,
            SessionRecord::new(s3, p2.clone(), serde_json::json!({"k": 3})),
        )
        .await
        .unwrap();

    // Upsert: re-put s1 with new body.
    store
        .put(
            s1,
            SessionRecord::new(s1, p1.clone(), serde_json::json!({"k": 11})),
        )
        .await
        .unwrap();
    let loaded = store.get(s1).await.unwrap().unwrap();
    assert_eq!(loaded.body, serde_json::json!({"k": 11}));

    // list(p1) returns s1+s2.
    let mut listed = store.list(p1).await.unwrap();
    listed.sort_by_key(|s| s.0);
    let mut expected = vec![s1, s2];
    expected.sort_by_key(|s| s.0);
    assert_eq!(listed, expected);

    // Missing session => None, not a backend error.
    assert!(store.get(SessionId::new()).await.unwrap().is_none());
}
