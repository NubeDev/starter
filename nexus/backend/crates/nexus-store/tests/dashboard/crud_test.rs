//! Dashboard + panel CRUD against real Postgres under the runtime role: tenant
//! isolation, slug uniqueness per tenant, and panel cascade on dashboard delete.

#![cfg(feature = "testing")]

use nexus_store::dashboard::{self, NewDashboard, NewPanel};
use nexus_store::testing::runtime_pool;
use serde_json::json;
use starter_store_postgres::testing::with_database;
use uuid::Uuid;

fn new_dash(slug: &str) -> NewDashboard {
    NewDashboard {
        slug: slug.into(),
        name: "Plant 1".into(),
    }
}

fn new_panel(dashboard_id: Uuid) -> NewPanel {
    NewPanel {
        dashboard_id,
        datasource_id: None,
        title: "Temp".into(),
        sql: "SELECT 1".into(),
        viz: "line".into(),
        layout: json!({"x": 0, "y": 0}),
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn dashboards_are_tenant_scoped_and_slug_unique() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let d = dashboard::insert(pg, "acme", &new_dash("plant-1"))
        .await
        .expect("insert");

    // Same slug in the same tenant is a conflict.
    let conflict = dashboard::insert(pg, "acme", &new_dash("plant-1")).await;
    assert!(matches!(conflict, Err(starter_spi::Error::Conflict { .. })));

    // The same slug in a *different* tenant is fine — slugs are per-tenant.
    dashboard::insert(pg, "globex", &new_dash("plant-1"))
        .await
        .expect("same slug, other tenant");

    // by_slug resolves within the tenant only.
    assert_eq!(
        dashboard::by_slug(pg, "acme", "plant-1")
            .await
            .unwrap()
            .unwrap()
            .id,
        d.id
    );
    assert_eq!(dashboard::list(pg, "acme").await.unwrap().len(), 1);
    assert_eq!(dashboard::list(pg, "globex").await.unwrap().len(), 1);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn panels_belong_to_a_dashboard_and_cascade_on_delete() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let d = dashboard::insert(pg, "acme", &new_dash("plant-1"))
        .await
        .unwrap();
    dashboard::panel::insert(pg, "acme", &new_panel(d.id))
        .await
        .unwrap();
    dashboard::panel::insert(pg, "acme", &new_panel(d.id))
        .await
        .unwrap();

    assert_eq!(
        dashboard::panel::list_for_dashboard(pg, "acme", d.id)
            .await
            .unwrap()
            .len(),
        2
    );

    // Deleting the dashboard cascades to its panels.
    assert!(dashboard::delete(pg, "acme", d.id).await.unwrap());
    assert_eq!(
        dashboard::panel::list_for_dashboard(pg, "acme", d.id)
            .await
            .unwrap()
            .len(),
        0
    );
}
