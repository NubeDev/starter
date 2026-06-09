//! Flow CRUD against real Postgres under the runtime role: tenant isolation,
//! name uniqueness per tenant, partial update, and the enabled toggle.

#![cfg(feature = "testing")]

use nexus_store::flow::{self, FlowPatch, NewFlow};
use nexus_store::testing::runtime_pool;
use serde_json::json;
use starter_store_postgres::testing::with_database;

fn new_flow(name: &str) -> NewFlow {
    NewFlow {
        name: name.into(),
        input: json!({ "type": "http_poll", "url": "https://x/", "interval": "15m" }),
        pipeline: json!([{ "type": "json_to_arrow" }]),
        output: json!({ "type": "postgres", "uri": "postgres://x", "table": "t" }),
        enabled: false,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn flows_are_tenant_scoped_and_name_unique() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let f = flow::insert(pg, "acme", &new_flow("weather")).await.unwrap();
    assert!(!f.enabled);

    // Same name, same tenant ⇒ conflict.
    let conflict = flow::insert(pg, "acme", &new_flow("weather")).await;
    assert!(matches!(conflict, Err(starter_spi::Error::Conflict { .. })));

    // Same name, other tenant ⇒ fine.
    flow::insert(pg, "globex", &new_flow("weather")).await.unwrap();

    // Tenant isolation on list + get.
    assert_eq!(flow::list(pg, "acme").await.unwrap().len(), 1);
    assert_eq!(flow::list(pg, "globex").await.unwrap().len(), 1);
    assert!(flow::get(pg, "globex", f.id).await.unwrap().is_none());
    assert_eq!(flow::get(pg, "acme", f.id).await.unwrap().unwrap().id, f.id);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn update_and_enable_toggle_apply() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;
    let f = flow::insert(pg, "acme", &new_flow("weather")).await.unwrap();

    // Partial update leaves untouched fields alone.
    flow::update(
        pg,
        "acme",
        f.id,
        &FlowPatch {
            name: Some("weather-eu".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let got = flow::get(pg, "acme", f.id).await.unwrap().unwrap();
    assert_eq!(got.name, "weather-eu");
    assert_eq!(got.input, f.input, "input unchanged by a name-only patch");

    // The enabled toggle the start/stop routes use.
    assert!(flow::set_enabled(pg, "acme", f.id, true).await.unwrap());
    assert!(flow::get(pg, "acme", f.id).await.unwrap().unwrap().enabled);

    assert!(flow::delete(pg, "acme", f.id).await.unwrap());
    assert!(flow::get(pg, "acme", f.id).await.unwrap().is_none());
}
