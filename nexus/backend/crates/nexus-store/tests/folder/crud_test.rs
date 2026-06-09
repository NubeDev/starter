//! Folder CRUD against real Postgres under the runtime role (WS-05): tenant
//! isolation, nesting, reparenting, the three-valued parent patch, and the
//! re-root-on-delete behaviour (children/dashboards are never destroyed).

#![cfg(feature = "testing")]

use nexus_store::dashboard::{self, NewDashboard};
use nexus_store::folder::{self, FolderPatch, NewFolder};
use nexus_store::testing::runtime_pool;
use starter_store_postgres::testing::with_database;

fn root(name: &str) -> NewFolder {
    NewFolder {
        parent_id: None,
        name: name.into(),
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn folders_are_tenant_scoped_and_nestable() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let parent = folder::insert(pg, "acme", &root("Plants")).await.unwrap();
    let child = folder::insert(
        pg,
        "acme",
        &NewFolder {
            parent_id: Some(parent.id),
            name: "Plant 1".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(child.parent_id, Some(parent.id));

    // Another tenant sees none of acme's folders.
    assert_eq!(folder::list(pg, "acme").await.unwrap().len(), 2);
    assert_eq!(folder::list(pg, "globex").await.unwrap().len(), 0);

    // A parent in another tenant is invisible, so filing under it is rejected.
    let cross = folder::insert(
        pg,
        "globex",
        &NewFolder {
            parent_id: Some(parent.id),
            name: "stolen".into(),
        },
    )
    .await;
    assert!(matches!(cross, Err(starter_spi::Error::Invalid { .. })));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn the_parent_patch_is_three_valued() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let a = folder::insert(pg, "acme", &root("A")).await.unwrap();
    let b = folder::insert(pg, "acme", &root("B")).await.unwrap();

    // Some(Some(parent)) moves B under A.
    let moved = folder::update(
        pg,
        "acme",
        b.id,
        &FolderPatch {
            parent_id: Some(Some(a.id)),
            ..FolderPatch::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(moved.parent_id, Some(a.id));

    // None leaves the parent untouched while renaming.
    let renamed = folder::update(
        pg,
        "acme",
        b.id,
        &FolderPatch {
            name: Some("B2".into()),
            parent_id: None,
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(renamed.parent_id, Some(a.id), "None leaves parent unchanged");
    assert_eq!(renamed.name, "B2");

    // Some(None) re-roots B.
    let rerooted = folder::update(
        pg,
        "acme",
        b.id,
        &FolderPatch {
            parent_id: Some(None),
            ..FolderPatch::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(rerooted.parent_id, None, "Some(None) re-roots");

    // A folder cannot be its own parent.
    let cycle = folder::update(
        pg,
        "acme",
        a.id,
        &FolderPatch {
            parent_id: Some(Some(a.id)),
            ..FolderPatch::default()
        },
    )
    .await;
    assert!(matches!(cycle, Err(starter_spi::Error::Invalid { .. })));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn deleting_a_folder_reroots_children_and_dashboards() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let parent = folder::insert(pg, "acme", &root("Plants")).await.unwrap();
    let child = folder::insert(
        pg,
        "acme",
        &NewFolder {
            parent_id: Some(parent.id),
            name: "Plant 1".into(),
        },
    )
    .await
    .unwrap();
    let dash = dashboard::insert(
        pg,
        "acme",
        &NewDashboard {
            slug: "p1".into(),
            name: "P1".into(),
            icon: "Activity".into(),
            accent: "152 76% 44%".into(),
            folder_id: Some(parent.id),
        },
    )
    .await
    .unwrap();

    // Deleting the parent re-roots, never destroys, its contents.
    assert!(folder::delete(pg, "acme", parent.id).await.unwrap());

    let child_after = folder::by_id(pg, "acme", child.id).await.unwrap().unwrap();
    assert_eq!(child_after.parent_id, None, "child re-rooted, not deleted");

    let dash_after = dashboard::by_slug(pg, "acme", "p1").await.unwrap().unwrap();
    assert_eq!(dash_after.folder_id, None, "dashboard re-rooted, not deleted");
    assert_eq!(dash_after.id, dash.id);

    // The folder itself is gone.
    assert!(folder::by_id(pg, "acme", parent.id).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn insert_with_id_restores_the_original_id() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let f = folder::insert(pg, "acme", &root("Keep")).await.unwrap();
    let id = f.id;
    assert!(folder::delete(pg, "acme", id).await.unwrap());

    // The undo path resurrects under the same id so any re-filing can target it.
    let resurrected = folder::insert_with_id(
        pg,
        "acme",
        id,
        &NewFolder {
            parent_id: None,
            name: "Keep".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(resurrected.id, id);
    assert!(folder::by_id(pg, "acme", id).await.unwrap().is_some());
}
