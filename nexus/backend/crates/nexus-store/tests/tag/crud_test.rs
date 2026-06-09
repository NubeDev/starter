//! Tag CRUD against real Postgres under the runtime role: set replaces the full
//! set, bare vs key:value tags round-trip, reverse lookup filters by value,
//! tenant isolation holds, and deleting an entity sweeps its tags.

#![cfg(feature = "testing")]

use nexus_store::dashboard::{self, NewDashboard};
use nexus_store::tag::{self, EntityRef, TagRecord};
use nexus_store::testing::runtime_pool;
use starter_store_postgres::testing::with_database;

fn entity(id: &str) -> EntityRef {
    EntityRef {
        entity_type: "dashboard".into(),
        entity_id: id.into(),
    }
}

fn bare(key: &str) -> TagRecord {
    TagRecord {
        key: key.into(),
        value: None,
    }
}

fn kv(key: &str, value: &str) -> TagRecord {
    TagRecord {
        key: key.into(),
        value: Some(value.into()),
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn set_replaces_the_full_set_and_both_shapes_round_trip() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let e = entity("dash-1");
    // A mix of bare labels and key:value tags — the user's two examples.
    tag::set_for_entity(pg, "acme", &e, &[bare("temp"), bare("site"), kv("building", "abc")])
        .await
        .expect("set");

    let mut got = tag::list_for_entity(pg, "acme", &e).await.unwrap();
    got.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(got, vec![kv("building", "abc"), bare("site"), bare("temp")]);

    // Set is a full replace: omitting `site`/`temp` drops them, and a new value
    // for `building` overwrites in place rather than duplicating.
    tag::set_for_entity(pg, "acme", &e, &[kv("building", "xyz"), kv("zone", "123")])
        .await
        .unwrap();
    let mut got = tag::list_for_entity(pg, "acme", &e).await.unwrap();
    got.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(got, vec![kv("building", "xyz"), kv("zone", "123")]);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn reverse_lookup_filters_by_key_and_optional_value() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    tag::set_for_entity(pg, "acme", &entity("a"), &[kv("building", "abc")])
        .await
        .unwrap();
    tag::set_for_entity(pg, "acme", &entity("b"), &[kv("building", "abc")])
        .await
        .unwrap();
    tag::set_for_entity(pg, "acme", &entity("c"), &[kv("building", "xyz")])
        .await
        .unwrap();

    // `None` value: every entity tagged with the key at all.
    let any = tag::entities_with_tag(pg, "acme", "dashboard", "building", None)
        .await
        .unwrap();
    assert_eq!(any.len(), 3);

    // `Some` value: pinned exactly.
    let abc = tag::entities_with_tag(pg, "acme", "dashboard", "building", Some("abc"))
        .await
        .unwrap();
    assert_eq!(abc.len(), 2);
    assert!(abc.iter().all(|x| x.entity_id == "a" || x.entity_id == "b"));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tags_are_tenant_scoped() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let e = entity("shared-id");
    tag::set_for_entity(pg, "acme", &e, &[bare("temp")])
        .await
        .unwrap();

    // The same entity id in another tenant has its own (empty) tag set, and a
    // reverse lookup in that tenant sees nothing.
    assert!(tag::list_for_entity(pg, "globex", &e).await.unwrap().is_empty());
    assert!(tag::entities_with_tag(pg, "globex", "dashboard", "temp", None)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn deleting_an_entity_sweeps_its_tags() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let d = dashboard::insert(pg, "acme", &NewDashboard { slug: "plant-1".into(), name: "P1".into() })
        .await
        .unwrap();
    let e = EntityRef {
        entity_type: "dashboard".into(),
        entity_id: d.id.to_string(),
    };
    tag::set_for_entity(pg, "acme", &e, &[bare("temp"), kv("building", "abc")])
        .await
        .unwrap();

    assert!(dashboard::delete(pg, "acme", d.id).await.unwrap());
    // The dashboard delete path swept the tags — no orphans left behind.
    assert!(tag::list_for_entity(pg, "acme", &e).await.unwrap().is_empty());
}
