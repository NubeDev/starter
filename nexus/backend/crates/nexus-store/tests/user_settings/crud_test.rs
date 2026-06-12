//! User-settings CRUD against real Postgres under the runtime role: an absent
//! row reads as `{}`, set upserts the whole bag, a second set replaces it, and
//! the bag is isolated per `(tenant, user)`.

#![cfg(feature = "testing")]

use nexus_store::testing::runtime_pool;
use nexus_store::user_settings;
use serde_json::json;
use starter_store_postgres::testing::with_database;

#[tokio::test]
#[ignore = "requires docker"]
async fn absent_row_reads_as_empty_object() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    // A user who has never saved settings gets `{}`, not an error or null.
    let got = user_settings::get(pg, "acme", "user-1").await.unwrap();
    assert_eq!(got, json!({}));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn set_upserts_and_replaces_the_whole_bag() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    // First save inserts. The bag is freeform; starred dashboards are one key.
    let first = json!({ "starredDashboards": ["d-1", "d-2"], "theme": "dark" });
    user_settings::set(pg, "acme", "user-1", &first)
        .await
        .unwrap();
    assert_eq!(
        user_settings::get(pg, "acme", "user-1").await.unwrap(),
        first
    );

    // Second save is a full replace (upsert), not a merge: the old `theme` key
    // is gone because the client sent the whole bag without it.
    let second = json!({ "starredDashboards": ["d-2"] });
    user_settings::set(pg, "acme", "user-1", &second)
        .await
        .unwrap();
    assert_eq!(
        user_settings::get(pg, "acme", "user-1").await.unwrap(),
        second
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn settings_are_isolated_per_user_and_tenant() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let mine = json!({ "starredDashboards": ["d-1"] });
    user_settings::set(pg, "acme", "user-1", &mine)
        .await
        .unwrap();

    // A different user in the same tenant has their own (empty) bag — one user's
    // stars never leak into another's.
    assert_eq!(
        user_settings::get(pg, "acme", "user-2").await.unwrap(),
        json!({})
    );
    // The same user id in another tenant is also isolated (RLS + the tenant key).
    assert_eq!(
        user_settings::get(pg, "globex", "user-1").await.unwrap(),
        json!({})
    );
}
