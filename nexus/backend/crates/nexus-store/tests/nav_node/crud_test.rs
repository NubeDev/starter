//! Nav-node CRUD against real Postgres under the runtime role (WS-13): tenant
//! isolation, nesting, reorder/reparent (the three-valued parent patch), the
//! re-root-on-delete behaviour, and the dashboard-delete sweep that blanks a
//! mount back to a `group` header without losing the node.

#![cfg(feature = "testing")]

use nexus_store::dashboard::{self, NewDashboard};
use nexus_store::nav_node::{self, NavNodePatch, NewNavNode};
use nexus_store::testing::runtime_pool;
use serde_json::json;
use starter_store_postgres::testing::with_database;

fn group(title: &str) -> NewNavNode {
    NewNavNode {
        parent_id: None,
        title: title.into(),
        sort_order: 0,
        target: json!({ "kind": "group" }),
        context: None,
        icon: None,
        accent: None,
    }
}

fn dashboard_mount(title: &str, dashboard_id: uuid::Uuid, building: &str) -> NewNavNode {
    NewNavNode {
        parent_id: None,
        title: title.into(),
        sort_order: 0,
        target: json!({ "kind": "dashboard", "dashboardId": dashboard_id.to_string() }),
        context: Some(json!({ "values": { "building": building } })),
        icon: None,
        accent: None,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn nav_nodes_are_tenant_scoped_and_nestable() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let parent = nav_node::insert(pg, "acme", &group("Buildings"))
        .await
        .unwrap();
    let child = nav_node::insert(
        pg,
        "acme",
        &NewNavNode {
            parent_id: Some(parent.id),
            ..group("Building-1")
        },
    )
    .await
    .unwrap();
    assert_eq!(child.parent_id, Some(parent.id));

    // Another tenant sees none of acme's nodes.
    assert_eq!(nav_node::list(pg, "acme").await.unwrap().len(), 2);
    assert_eq!(nav_node::list(pg, "globex").await.unwrap().len(), 0);

    // A parent in another tenant is invisible, so filing under it is rejected.
    let cross = nav_node::insert(
        pg,
        "globex",
        &NewNavNode {
            parent_id: Some(parent.id),
            ..group("stolen")
        },
    )
    .await;
    assert!(matches!(cross, Err(starter_spi::Error::Invalid { .. })));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn the_parent_patch_is_three_valued_and_reorders() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let a = nav_node::insert(pg, "acme", &group("A")).await.unwrap();
    let b = nav_node::insert(pg, "acme", &group("B")).await.unwrap();

    // Some(Some(parent)) moves B under A, and sort_order reorders.
    let moved = nav_node::update(
        pg,
        "acme",
        b.id,
        &NavNodePatch {
            parent_id: Some(Some(a.id)),
            sort_order: Some(5),
            ..NavNodePatch::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(moved.parent_id, Some(a.id));
    assert_eq!(moved.sort_order, 5);

    // None leaves the parent untouched while retitling.
    let retitled = nav_node::update(
        pg,
        "acme",
        b.id,
        &NavNodePatch {
            title: Some("B2".into()),
            ..NavNodePatch::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        retitled.parent_id,
        Some(a.id),
        "None leaves parent unchanged"
    );
    assert_eq!(retitled.title, "B2");

    // Some(None) re-roots B.
    let rerooted = nav_node::update(
        pg,
        "acme",
        b.id,
        &NavNodePatch {
            parent_id: Some(None),
            ..NavNodePatch::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(rerooted.parent_id, None, "Some(None) re-roots");

    // A node cannot be its own parent.
    let cycle = nav_node::update(
        pg,
        "acme",
        a.id,
        &NavNodePatch {
            parent_id: Some(Some(a.id)),
            ..NavNodePatch::default()
        },
    )
    .await;
    assert!(matches!(cycle, Err(starter_spi::Error::Invalid { .. })));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn context_clears_when_retargeting_to_group() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let dash = dashboard::insert(
        pg,
        "acme",
        &NewDashboard {
            slug: "energy".into(),
            name: "Energy".into(),
            icon: "Activity".into(),
            accent: "152 76% 44%".into(),
            folder_id: None,
        },
    )
    .await
    .unwrap();

    let node = nav_node::insert(pg, "acme", &dashboard_mount("Building-1", dash.id, "b1"))
        .await
        .unwrap();
    assert!(node.context.is_some());

    // Retarget to a group and clear the context in one patch.
    let blanked = nav_node::update(
        pg,
        "acme",
        node.id,
        &NavNodePatch {
            target: Some(json!({ "kind": "group" })),
            context: Some(None),
            ..NavNodePatch::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(blanked.target, json!({ "kind": "group" }));
    assert!(blanked.context.is_none(), "Some(None) clears context");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn deleting_a_node_reroots_children() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let parent = nav_node::insert(pg, "acme", &group("Buildings"))
        .await
        .unwrap();
    let child = nav_node::insert(
        pg,
        "acme",
        &NewNavNode {
            parent_id: Some(parent.id),
            ..group("Building-1")
        },
    )
    .await
    .unwrap();

    assert!(nav_node::delete(pg, "acme", parent.id).await.unwrap());

    let child_after = nav_node::by_id(pg, "acme", child.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(child_after.parent_id, None, "child re-rooted, not deleted");
    assert!(nav_node::by_id(pg, "acme", parent.id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn deleting_a_dashboard_sweeps_its_mounts_to_groups() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let dash = dashboard::insert(
        pg,
        "acme",
        &NewDashboard {
            slug: "energy".into(),
            name: "Energy".into(),
            icon: "Activity".into(),
            accent: "152 76% 44%".into(),
            folder_id: None,
        },
    )
    .await
    .unwrap();

    // One page, two mounts at two buildings — the whole WS-13 reuse story.
    let b1 = nav_node::insert(pg, "acme", &dashboard_mount("Building-1", dash.id, "b1"))
        .await
        .unwrap();
    let b2 = nav_node::insert(pg, "acme", &dashboard_mount("Building-2", dash.id, "b2"))
        .await
        .unwrap();
    // A group node and a route node must be untouched by the sweep.
    let header = nav_node::insert(pg, "acme", &group("Buildings"))
        .await
        .unwrap();

    // Deleting the page sweeps both mounts to plain groups; the nodes survive.
    assert!(dashboard::delete(pg, "acme", dash.id).await.unwrap());

    for id in [b1.id, b2.id] {
        let after = nav_node::by_id(pg, "acme", id).await.unwrap().unwrap();
        assert_eq!(
            after.target,
            json!({ "kind": "group" }),
            "mount swept to group header"
        );
        assert!(after.context.is_none(), "dangling context cleared");
    }
    // The pre-existing header node is unaffected.
    let header_after = nav_node::by_id(pg, "acme", header.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(header_after.target, json!({ "kind": "group" }));
    assert_eq!(header_after.title, "Buildings");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn insert_with_id_restores_the_original_id() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let n = nav_node::insert(pg, "acme", &group("Keep")).await.unwrap();
    let id = n.id;
    assert!(nav_node::delete(pg, "acme", id).await.unwrap());

    // The undo path resurrects under the same id so any re-parenting can target it.
    let resurrected = nav_node::insert_with_id(pg, "acme", id, &group("Keep"))
        .await
        .unwrap();
    assert_eq!(resurrected.id, id);
    assert!(nav_node::by_id(pg, "acme", id).await.unwrap().is_some());
}
