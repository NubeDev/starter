//! ADR-tenant-hierarchy — store-side closure maintenance + subtree
//! queries (sqlite). Verifies that create_tenant builds the closure
//! correctly, subtree_ids / is_ancestor read it back, and the
//! parent-not-found + depth-cap guards fire.

#![cfg(feature = "sqlite")]

use starter_auth_users::store::{
    SqliteTenantStore, TenantRecord, TenantStore, TenantStoreError,
};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");

async fn fresh_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter_auth_users",
            migrator: &AUTH_USERS_MIGRATOR,
        })
        .run()
        .await
        .expect("migrations apply");
    pool
}

fn tenant(id: &str, slug: &str, parent: Option<&str>) -> TenantRecord {
    TenantRecord {
        id: id.into(),
        slug: slug.into(),
        display_name: slug.into(),
        audit_allow_sample: None,
        parent_id: parent.map(str::to_owned),
    }
}

/// Build the ADR example tree and assert closure-derived queries.
///
///   daikin
///   ├── acme
///   │   ├── acme-north
///   │   └── acme-south
///   └── byco
async fn seed_tree(store: &SqliteTenantStore) {
    store.create_tenant(&tenant("daikin", "daikin", None)).await.unwrap();
    store.create_tenant(&tenant("acme", "acme", Some("daikin"))).await.unwrap();
    store.create_tenant(&tenant("acme-north", "acme-north", Some("acme"))).await.unwrap();
    store.create_tenant(&tenant("acme-south", "acme-south", Some("acme"))).await.unwrap();
    store.create_tenant(&tenant("byco", "byco", Some("daikin"))).await.unwrap();
}

#[tokio::test]
async fn subtree_ids_returns_self_and_all_descendants() {
    let store = SqliteTenantStore::new(fresh_pool().await);
    seed_tree(&store).await;

    let mut daikin = store.subtree_ids("daikin").await.unwrap();
    daikin.sort();
    assert_eq!(
        daikin,
        vec!["acme", "acme-north", "acme-south", "byco", "daikin"]
    );

    let mut acme = store.subtree_ids("acme").await.unwrap();
    acme.sort();
    assert_eq!(acme, vec!["acme", "acme-north", "acme-south"]);

    // A leaf is its own (singleton) subtree.
    assert_eq!(store.subtree_ids("acme-north").await.unwrap(), vec!["acme-north"]);

    // Unknown tenant → empty.
    assert!(store.subtree_ids("nope").await.unwrap().is_empty());
}

#[tokio::test]
async fn is_ancestor_reflects_the_tree() {
    let store = SqliteTenantStore::new(fresh_pool().await);
    seed_tree(&store).await;

    // Self is an ancestor of itself (depth 0).
    assert!(store.is_ancestor("daikin", "daikin").await.unwrap());
    // Transitive ancestor (2 levels).
    assert!(store.is_ancestor("daikin", "acme-north").await.unwrap());
    assert!(store.is_ancestor("acme", "acme-south").await.unwrap());
    // Not an ancestor: upward, sideways, unrelated.
    assert!(!store.is_ancestor("acme", "daikin").await.unwrap());
    assert!(!store.is_ancestor("acme", "byco").await.unwrap());
    assert!(!store.is_ancestor("acme-north", "acme-south").await.unwrap());
}

#[tokio::test]
async fn list_children_is_direct_only() {
    let store = SqliteTenantStore::new(fresh_pool().await);
    seed_tree(&store).await;

    let mut kids: Vec<String> = store
        .list_children("daikin")
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect();
    kids.sort();
    // Direct children only — acme + byco, NOT acme-north/south.
    assert_eq!(kids, vec!["acme", "byco"]);

    let mut acme_kids: Vec<String> = store
        .list_children("acme")
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect();
    acme_kids.sort();
    assert_eq!(acme_kids, vec!["acme-north", "acme-south"]);
}

#[tokio::test]
async fn list_subtree_returns_full_records_root_first() {
    let store = SqliteTenantStore::new(fresh_pool().await);
    seed_tree(&store).await;

    let sub = store.list_subtree("acme").await.unwrap();
    // root (acme) first by depth.
    assert_eq!(sub.first().unwrap().id, "acme");
    let ids: std::collections::BTreeSet<_> = sub.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(
        ids,
        ["acme", "acme-north", "acme-south"].into_iter().collect()
    );
    // parent_id round-trips.
    let north = sub.iter().find(|t| t.id == "acme-north").unwrap();
    assert_eq!(north.parent_id.as_deref(), Some("acme"));
}

#[tokio::test]
async fn create_under_missing_parent_is_parent_not_found() {
    let store = SqliteTenantStore::new(fresh_pool().await);
    let err = store
        .create_tenant(&tenant("orphan", "orphan", Some("ghost")))
        .await
        .unwrap_err();
    assert!(
        matches!(err, TenantStoreError::ParentNotFound(_)),
        "expected ParentNotFound, got {err:?}"
    );
    // And nothing was written (no self closure row).
    assert!(store.subtree_ids("orphan").await.unwrap().is_empty());
}

#[tokio::test]
async fn depth_cap_is_enforced() {
    let store = SqliteTenantStore::new(fresh_pool().await);
    // Chain tenants until the store refuses. MAX_TENANT_DEPTH is 16,
    // so node at depth 16 (the 17th) must be refused.
    let mut last: Option<String> = None;
    let mut created = 0u32;
    for i in 0..64 {
        let id = format!("n{i}");
        let res = store
            .create_tenant(&tenant(&id, &format!("n{i}"), last.as_deref()))
            .await;
        match res {
            Ok(()) => {
                created += 1;
                last = Some(id);
            }
            Err(TenantStoreError::MaxDepthExceeded(_)) => {
                // Hit the cap. Should have created exactly
                // MAX_TENANT_DEPTH nodes (depths 0..=15).
                assert_eq!(created, 16, "expected 16 nodes before the cap");
                return;
            }
            Err(e) => panic!("unexpected error at depth {i}: {e:?}"),
        }
    }
    panic!("depth cap never fired after {created} nodes");
}

#[tokio::test]
async fn parent_id_is_immutable() {
    let store = SqliteTenantStore::new(fresh_pool().await);
    seed_tree(&store).await;
    let pool = store.pool().clone();

    // Try to re-parent acme-north under byco by hand. The trigger
    // must refuse.
    let res = sqlx::query(
        "UPDATE starter_auth_users_tenants SET parent_id = 'byco' WHERE id = 'acme-north'",
    )
    .execute(pool.sqlx())
    .await;
    assert!(res.is_err(), "re-parent UPDATE should be refused by trigger");
    assert!(
        res.unwrap_err().to_string().contains("immutable"),
        "trigger message should mention 'immutable'"
    );
}
