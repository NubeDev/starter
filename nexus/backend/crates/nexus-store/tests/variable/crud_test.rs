//! Dashboard-variable CRUD against real Postgres under the runtime role: tenant
//! isolation, name uniqueness per dashboard, partial update of the selection,
//! the `current` jsonb round-trip, and cascade on dashboard delete.

#![cfg(feature = "testing")]

use nexus_store::dashboard::{self, NewDashboard};
use nexus_store::testing::runtime_pool;
use nexus_store::variable::{self, NewVariable, VariablePatch};
use serde_json::json;
use starter_store_postgres::testing::with_database;
use uuid::Uuid;

fn new_dash(slug: &str) -> NewDashboard {
    NewDashboard {
        slug: slug.into(),
        name: "Plant 1".into(),
        icon: "Activity".into(),
        accent: "152 76% 44%".into(),
        folder_id: None,
    }
}

fn new_var(dashboard_id: Uuid, name: &str) -> NewVariable {
    NewVariable {
        dashboard_id,
        name: name.into(),
        label: Some("Region".into()),
        kind: "custom".into(),
        options_config: json!({"options": ["a", "b"]}),
        current: vec!["a".into()],
        multi: false,
        include_all: false,
        hidden: false,
        sort_order: 0,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn variables_are_tenant_scoped_and_name_unique_per_dashboard() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let d = dashboard::insert(pg, "acme", &new_dash("plant-1"))
        .await
        .unwrap();
    let v = variable::insert(pg, "acme", &new_var(d.id, "region"))
        .await
        .expect("insert");
    assert_eq!(
        v.current,
        vec!["a".to_string()],
        "current jsonb round-trips"
    );
    assert_eq!(v.options_config, json!({"options": ["a", "b"]}));

    // Same name on the same dashboard is a conflict.
    let conflict = variable::insert(pg, "acme", &new_var(d.id, "region")).await;
    assert!(matches!(conflict, Err(starter_spi::Error::Conflict { .. })));

    // A second dashboard may reuse the name — uniqueness is per dashboard.
    let d2 = dashboard::insert(pg, "acme", &new_dash("plant-2"))
        .await
        .unwrap();
    variable::insert(pg, "acme", &new_var(d2.id, "region"))
        .await
        .expect("same name, other dashboard");

    // Listing is scoped to the dashboard and the tenant.
    assert_eq!(
        variable::list_for_dashboard(pg, "acme", d.id)
            .await
            .unwrap()
            .len(),
        1
    );
    // Another tenant sees none of acme's variables (RLS).
    assert_eq!(
        variable::list_for_dashboard(pg, "globex", d.id)
            .await
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn variable_update_is_partial_and_tenant_scoped() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let d = dashboard::insert(pg, "acme", &new_dash("plant-1"))
        .await
        .unwrap();
    let v = variable::insert(pg, "acme", &new_var(d.id, "region"))
        .await
        .unwrap();

    // The common path: a `current`-only patch (the user picked a new value in
    // the bar) leaves name/kind/options untouched (COALESCE).
    let patch = VariablePatch {
        current: Some(vec!["b".into(), "c".into()]),
        multi: Some(true),
        ..VariablePatch::default()
    };
    let updated = variable::update(pg, "acme", v.id, &patch)
        .await
        .unwrap()
        .expect("variable visible to its tenant");
    assert_eq!(updated.current, vec!["b".to_string(), "c".to_string()]);
    assert!(updated.multi);
    assert_eq!(updated.name, "region", "omitted field unchanged");
    assert_eq!(updated.kind, "custom", "omitted field unchanged");

    // Another tenant cannot update the variable — RLS hides it, so the update
    // matches no row and returns None rather than mutating across tenants.
    let cross = variable::update(pg, "globex", v.id, &patch).await.unwrap();
    assert!(cross.is_none(), "cross-tenant update finds no variable");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn variables_cascade_on_dashboard_delete() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let d = dashboard::insert(pg, "acme", &new_dash("plant-1"))
        .await
        .unwrap();
    variable::insert(pg, "acme", &new_var(d.id, "region"))
        .await
        .unwrap();
    variable::insert(pg, "acme", &new_var(d.id, "building"))
        .await
        .unwrap();
    assert_eq!(
        variable::list_for_dashboard(pg, "acme", d.id)
            .await
            .unwrap()
            .len(),
        2
    );

    // Deleting the dashboard cascades to its variables (FK ON DELETE CASCADE).
    assert!(dashboard::delete(pg, "acme", d.id).await.unwrap());
    assert_eq!(
        variable::list_for_dashboard(pg, "acme", d.id)
            .await
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn delete_returns_whether_a_row_matched() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let d = dashboard::insert(pg, "acme", &new_dash("plant-1"))
        .await
        .unwrap();
    let v = variable::insert(pg, "acme", &new_var(d.id, "region"))
        .await
        .unwrap();

    // Wrong tenant: RLS hides the row, so the delete matches nothing.
    assert!(!variable::delete(pg, "globex", v.id).await.unwrap());
    // Correct tenant removes it; a second delete is a no-op.
    assert!(variable::delete(pg, "acme", v.id).await.unwrap());
    assert!(!variable::delete(pg, "acme", v.id).await.unwrap());
}
