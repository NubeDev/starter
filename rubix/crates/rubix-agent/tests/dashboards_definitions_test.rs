//! Integration coverage for Goal-1 Phase A.1 — `PgDashboardStore`
//! round-trips against the `dashboards_definitions` table.
//!
//! Spins an ephemeral Postgres, applies the rubix dashboards
//! migration source, and exercises:
//!
//! 1. `insert_revision` round-trips (page lands as the live head).
//! 2. A second `insert_revision` for the same `page_id` supersedes
//!    the first; `get_active` returns only the new row.
//! 3. `list_active` returns the head only, scoped to the caller's
//!    tenant; `history` returns every revision in `created_at DESC`.
//! 4. `mark_superseded` flips the live row to superseded and
//!    `get_active` then returns `None` (delete path).

use rubix_spi::dashboard::{DashboardStore, ListFilter, NewRevision};
use rubix_store_postgres::{PgDashboardStore, DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE};
use starter_store_postgres::{migrate, testing::with_database};

fn sample(page_id: &str, tenant: &str, title: &str) -> NewRevision {
    NewRevision {
        page_id: page_id.to_string(),
        tenant_id: tenant.to_string(),
        owner_principal: "op@example.com".to_string(),
        title: title.to_string(),
        tags: vec!["energy".to_string()],
        body_json: serde_json::json!({"kind": "page", "children": []}),
        created_by: "op@example.com".to_string(),
    }
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn insert_supersede_list_history_delete_round_trip() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply dashboards_definitions migration");

    let store = PgDashboardStore::new(pool.clone());

    // 1) Insert v1 — round-trips.
    let v1 = store
        .insert_revision(sample("dashboard.test", "tenant-a", "v1"))
        .await
        .expect("insert v1");
    assert_eq!(v1.page_id, "dashboard.test");
    assert!(v1.superseded_at.is_none());
    let active = store
        .get_active("tenant-a", "dashboard.test")
        .await
        .expect("get active v1");
    assert_eq!(active.as_ref().map(|r| r.revision_id.clone()), Some(v1.revision_id.clone()));

    // 2) Insert v2 — supersedes v1.
    let v2 = store
        .insert_revision(sample("dashboard.test", "tenant-a", "v2"))
        .await
        .expect("insert v2");
    assert_ne!(v1.revision_id, v2.revision_id);
    let active2 = store
        .get_active("tenant-a", "dashboard.test")
        .await
        .expect("get active v2")
        .expect("v2 row present");
    assert_eq!(active2.revision_id, v2.revision_id);
    assert_eq!(active2.title, "v2");

    // 3) list_active scopes to tenant + returns one row; history
    //    returns both revisions newest-first.
    let _other = store
        .insert_revision(sample("dashboard.other", "tenant-b", "other"))
        .await
        .expect("insert other tenant");
    let listed = store
        .list_active("tenant-a", &ListFilter::default())
        .await
        .expect("list_active tenant-a");
    assert_eq!(listed.len(), 1, "only one live row in tenant-a");
    assert_eq!(listed[0].revision_id, v2.revision_id);

    let by_tag = store
        .list_active(
            "tenant-a",
            &ListFilter {
                tags_any: vec!["energy".to_string()],
                ..Default::default()
            },
        )
        .await
        .expect("list_active by tag");
    assert_eq!(by_tag.len(), 1);
    let miss_tag = store
        .list_active(
            "tenant-a",
            &ListFilter {
                tags_any: vec!["nope".to_string()],
                ..Default::default()
            },
        )
        .await
        .expect("list_active by missing tag");
    assert!(miss_tag.is_empty());

    let hist = store.history("dashboard.test").await.expect("history");
    assert_eq!(hist.len(), 2, "two revisions for dashboard.test");
    assert_eq!(hist[0].revision_id, v2.revision_id, "newest first");
    assert_eq!(hist[1].revision_id, v1.revision_id);

    // 4) mark_superseded — delete path leaves no live row.
    let updated = store
        .mark_superseded("tenant-a", "dashboard.test")
        .await
        .expect("mark superseded");
    assert_eq!(updated, 1);
    let after = store
        .get_active("tenant-a", "dashboard.test")
        .await
        .expect("get after delete");
    assert!(after.is_none());
}
