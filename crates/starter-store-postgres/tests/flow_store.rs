//! Integration tests for [`PgFlowStore`]. Twin of the
//! `flow_store_*` cases in `starter-store-sqlite/tests/flow.rs`.
//!
//! Marked `#[ignore]` because it requires Docker on the host (same
//! pattern as the agent-session / skills / migrate tests). CI runs
//! via `cargo test -p starter-store-postgres --features
//! "flow testing" -- --ignored`.

#![cfg(all(feature = "flow", feature = "testing"))]

use starter_flow_spi::flow::{FlowId, FlowRevision, FlowRevisionId, FlowStore};
use starter_store_postgres::flow::{PgFlowStore, FLOW_MIGRATION_SOURCE};
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

fn fresh_flow_id() -> FlowId {
    FlowId::new("com.acme.test.flow").unwrap()
}

#[tokio::test]
#[ignore = "requires docker"]
async fn flow_store_put_load_head_revisions() {
    let (pool, _guard) = boot().await;
    let store = PgFlowStore::new(pool);

    let fid = fresh_flow_id();
    let rev_a = FlowRevisionId::new();
    let rev_b = FlowRevisionId::new();

    store
        .put(FlowRevision::new(
            fid.clone(),
            rev_a,
            serde_json::json!({"v": 1}),
        ))
        .await
        .unwrap();
    store
        .put(FlowRevision::new(
            fid.clone(),
            rev_b,
            serde_json::json!({"v": 2}),
        ))
        .await
        .unwrap();

    // Head reflects the most recently put revision.
    assert_eq!(store.head(fid.clone()).await.unwrap(), Some(rev_b));

    // load(None) resolves to head.
    let loaded = store.load(fid.clone(), None).await.unwrap();
    assert_eq!(loaded.revision_id, rev_b);
    assert_eq!(loaded.body, serde_json::json!({"v": 2}));

    // Explicit load of the older revision still works (immutable).
    let loaded_a = store.load(fid.clone(), Some(rev_a)).await.unwrap();
    assert_eq!(loaded_a.body, serde_json::json!({"v": 1}));

    // Both revisions listed.
    let revs = store.revisions(fid.clone()).await.unwrap();
    assert_eq!(revs.len(), 2);
    assert!(revs.contains(&rev_a) && revs.contains(&rev_b));

    // Flow shows up in list().
    let flows = store.list().await.unwrap();
    assert_eq!(flows, vec![fid]);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn flow_revision_source_round_trips() {
    let (pool, _guard) = boot().await;
    let store = PgFlowStore::new(pool);

    let fid = fresh_flow_id();
    let rev = FlowRevisionId::new();
    store
        .put(FlowRevision::new(fid.clone(), rev, serde_json::json!({"v": 1})).with_source("cli"))
        .await
        .unwrap();
    let loaded = store.load(fid, Some(rev)).await.unwrap();
    assert_eq!(loaded.source, "cli");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn flow_put_is_idempotent_on_existing_revision() {
    // Re-putting the same (flow_id, revision_id) pair is a no-op
    // on the revisions table (immutable per SCOPE) but still
    // advances the head pointer. This mirrors the SQLite twin.
    let (pool, _guard) = boot().await;
    let store = PgFlowStore::new(pool);

    let fid = fresh_flow_id();
    let rev = FlowRevisionId::new();
    store
        .put(FlowRevision::new(
            fid.clone(),
            rev,
            serde_json::json!({"v": 1}),
        ))
        .await
        .unwrap();
    // Body in the second put would be ignored on conflict — assert
    // the original body survives.
    store
        .put(FlowRevision::new(
            fid.clone(),
            rev,
            serde_json::json!({"v": 99}),
        ))
        .await
        .unwrap();
    let loaded = store.load(fid, Some(rev)).await.unwrap();
    assert_eq!(loaded.body, serde_json::json!({"v": 1}));
}
