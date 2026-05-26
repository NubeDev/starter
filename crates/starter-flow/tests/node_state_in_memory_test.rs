//! Parameterised matrix for [`InMemoryNodeStateStore`]. The sister
//! file `starter-store-sqlite/tests/node_state_sqlite_test.rs` runs
//! the same scenarios against [`SqliteNodeStateStore`] so the two
//! impls stay observably identical.

use starter_flow::state::in_memory::InMemoryNodeStateStore;
use starter_flow_spi::flow::FlowId;
use starter_flow_spi::node::NodeId;
use starter_flow_spi::state::{NodeStateError, NodeStateKey, NodeStateStore};

fn k(name: &str) -> NodeStateKey {
    NodeStateKey::new(
        FlowId::new("acme.flows.demo").unwrap(),
        NodeId::new("acme.nodes.counter").unwrap(),
        name,
    )
    .unwrap()
}

#[tokio::test]
async fn get_missing_returns_none() {
    let store = InMemoryNodeStateStore::new();
    assert!(store.get(&k("count")).await.unwrap().is_none());
}

#[tokio::test]
async fn get_after_put_returns_value_and_version_one() {
    let store = InMemoryNodeStateStore::new();
    let v = store.put(&k("count"), b"42".to_vec()).await.unwrap();
    assert_eq!(v, 1);
    let got = store.get(&k("count")).await.unwrap().unwrap();
    assert_eq!(got.bytes, b"42");
    assert_eq!(got.version, 1);
}

#[tokio::test]
async fn put_overwrites_bumps_version() {
    let store = InMemoryNodeStateStore::new();
    store.put(&k("count"), b"1".to_vec()).await.unwrap();
    let v2 = store.put(&k("count"), b"2".to_vec()).await.unwrap();
    assert_eq!(v2, 2);
    let got = store.get(&k("count")).await.unwrap().unwrap();
    assert_eq!(got.bytes, b"2");
    assert_eq!(got.version, 2);
}

#[tokio::test]
async fn cas_success_against_current_version() {
    let store = InMemoryNodeStateStore::new();
    let v1 = store.put(&k("count"), b"1".to_vec()).await.unwrap();
    let v2 = store.cas(&k("count"), v1, b"2".to_vec()).await.unwrap();
    assert_eq!(v2, 2);
    let v3 = store.cas(&k("count"), 0, b"x".to_vec()).await;
    assert!(matches!(v3, Err(NodeStateError::CasMismatch { .. })));
}

#[tokio::test]
async fn cas_mismatch_returns_actual_version() {
    let store = InMemoryNodeStateStore::new();
    store.put(&k("count"), b"1".to_vec()).await.unwrap();
    let err = store.cas(&k("count"), 99, b"x".to_vec()).await.unwrap_err();
    match err {
        NodeStateError::CasMismatch { expected, actual } => {
            assert_eq!(expected, 99);
            assert_eq!(actual, Some(1));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn cas_initial_requires_zero_expected() {
    let store = InMemoryNodeStateStore::new();
    let v = store.cas(&k("count"), 0, b"hi".to_vec()).await.unwrap();
    assert_eq!(v, 1);
}

#[tokio::test]
async fn delete_then_get_missing() {
    let store = InMemoryNodeStateStore::new();
    store.put(&k("count"), b"1".to_vec()).await.unwrap();
    store.delete(&k("count")).await.unwrap();
    assert!(store.get(&k("count")).await.unwrap().is_none());
    // Delete-of-absent is a no-op.
    store.delete(&k("count")).await.unwrap();
}
