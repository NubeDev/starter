//! Smoke tests for the `dimensions` feature: migrations apply
//! against an empty Postgres testcontainer and the per-table CRUD
//! helpers round-trip.

#![cfg(all(feature = "dimensions", feature = "testing"))]

use starter_store_postgres::dimensions::{
    entities, entity_refs, ext_manifest_approvals, tag_definitions, DIMENSIONS_MIGRATION_SOURCE,
};
use starter_store_postgres::{migrate, testing::with_database, testing::ContainerGuard, Pool};
use starter_tags::{TagDefinition, TagKind};

async fn boot() -> (Pool, ContainerGuard) {
    let (pool, guard) = with_database().await;
    migrate(&pool)
        .with_source(DIMENSIONS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("dimensions migrations apply");
    (pool, guard)
}

#[tokio::test]
#[ignore = "requires docker"]
async fn dimensions_migrations_use_dedicated_version_table() {
    let (pool, _g) = boot().await;
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM _sqlx_migrations_dimensions")
        .fetch_one(pool.sqlx())
        .await
        .expect("version table exists");
    // 8 migrations expected (0001..0008).
    assert_eq!(count, 8);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn entities_and_refs_round_trip() {
    let (pool, _g) = boot().await;
    let e1 = entities::upsert(
        &pool,
        "ent_a",
        "equip",
        Some("AHU-1"),
        &serde_json::json!({"kind": "equip", "site": "HQ"}),
    )
    .await
    .unwrap();
    assert_eq!(e1.kind, "equip");

    entities::upsert(
        &pool,
        "ent_b",
        "point",
        Some("AHU-1.discharge"),
        &serde_json::json!({"kind": "point"}),
    )
    .await
    .unwrap();

    entity_refs::insert(&pool, "ent_b", "equipRef", "ent_a")
        .await
        .unwrap();
    // Idempotent on the composite PK.
    entity_refs::insert(&pool, "ent_b", "equipRef", "ent_a")
        .await
        .unwrap();

    let outgoing = entity_refs::list_from(&pool, "ent_b").await.unwrap();
    assert_eq!(outgoing.len(), 1);
    let incoming = entity_refs::list_to(&pool, "ent_a").await.unwrap();
    assert_eq!(incoming.len(), 1);

    // FK CASCADE: deleting ent_a removes the ref.
    entities::delete(&pool, "ent_a").await.unwrap();
    let incoming = entity_refs::list_to(&pool, "ent_a").await.unwrap();
    assert!(incoming.is_empty());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tag_definitions_round_trip_through_starter_tags_types() {
    let (pool, _g) = boot().await;
    let def = TagDefinition {
        key: "celsius".into(),
        kind: TagKind::NumDiscriminant,
        description: Some("temperature".into()),
        enum_values: None,
        ref_kind: None,
        source: "builtin".into(),
    };
    tag_definitions::upsert(&pool, &def).await.unwrap();
    let fetched = tag_definitions::get(&pool, "celsius")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.kind, TagKind::NumDiscriminant);
    assert_eq!(fetched.description.as_deref(), Some("temperature"));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn ext_manifest_approval_idempotent() {
    let (pool, _g) = boot().await;
    ext_manifest_approvals::approve(&pool, "ext.energy", "deadbeef", "install:initial")
        .await
        .unwrap();
    // Second insert is a no-op (ON CONFLICT DO NOTHING).
    ext_manifest_approvals::approve(&pool, "ext.energy", "deadbeef", "user:alice")
        .await
        .unwrap();

    assert!(
        ext_manifest_approvals::is_approved(&pool, "ext.energy", "deadbeef")
            .await
            .unwrap()
    );
    assert!(
        !ext_manifest_approvals::is_approved(&pool, "ext.energy", "feed")
            .await
            .unwrap()
    );

    let rows = ext_manifest_approvals::list(&pool, "ext.energy")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    // First-write wins on conflict — approved_by stays as install:initial.
    assert_eq!(rows[0].approved_by, "install:initial");
}
