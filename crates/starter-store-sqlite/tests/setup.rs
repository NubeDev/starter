//! P0 acceptance for the SQLite setup stores: catalog CRUD, the
//! tenant-over-global overlay (DOCS §5), and the run-index lifecycle
//! incl. the §8b resume-cursor + `list_open` semantics.

#![cfg(all(feature = "setup", feature = "testing"))]

use starter_flow_spi::flow::RunId;
use starter_setup_spi::envelope::TemplateEnvelope;
use starter_setup_spi::model::{
    Progress, SemVer, SetupRun, SetupRunStatus, TemplateId, TemplateSource,
};
use starter_setup_spi::store::{
    SetupRunFilter, SetupRunStore, TemplateFilter, TemplateStore, GLOBAL_TENANT_SENTINEL,
};
use starter_store_sqlite::setup::{SqliteSetupRunStore, SqliteTemplateStore, SETUP_MIGRATION_SOURCE};
use starter_store_sqlite::{migrate, testing::ephemeral, Pool};

const SAMPLE: &str = r#"
id: com.acme.add-device
version: 1.0.0
display_name: Add a device
category: Provisioning
input_schema: { type: object }
flow:
  nodes:
    - { id: com.acme.notify, kind: starter.flow.tool-call }
  links: []
"#;

async fn boot_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(SETUP_MIGRATION_SOURCE)
        .run()
        .await
        .expect("setup migrations apply");
    pool
}

fn template(tenant: Option<&str>, version: &str) -> starter_setup_spi::model::Template {
    let env = TemplateEnvelope::from_yaml(SAMPLE).unwrap();
    let mut t = env
        .into_template(tenant.map(str::to_string), TemplateSource::Api)
        .unwrap();
    t.version = SemVer::parse(version).unwrap();
    t
}

#[tokio::test]
async fn template_crud_and_latest() {
    let pool = boot_pool().await;
    let store = SqliteTemplateStore::new(pool);

    store.put(template(Some("acme"), "1.0.0")).await.unwrap();
    store.put(template(Some("acme"), "1.2.0")).await.unwrap();

    let id = TemplateId::from("com.acme.add-device");
    // Exact version.
    let got = store
        .get(Some("acme"), &id, Some(SemVer::new(1, 0, 0)))
        .await
        .unwrap()
        .expect("exists");
    assert_eq!(got.version, SemVer::new(1, 0, 0));
    // Latest.
    let latest = store.get(Some("acme"), &id, None).await.unwrap().unwrap();
    assert_eq!(latest.version, SemVer::new(1, 2, 0));

    // Delete one version.
    store
        .delete(Some("acme"), &id, SemVer::new(1, 0, 0))
        .await
        .unwrap();
    assert!(store
        .get(Some("acme"), &id, Some(SemVer::new(1, 0, 0)))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn tenant_row_overrides_global() {
    let pool = boot_pool().await;
    let store = SqliteTemplateStore::new(pool);

    // Global (extension) row + a tenant override at the same (id, version).
    let mut global = template(None, "1.0.0");
    global.display_name = "Global Add".into();
    store.put(global).await.unwrap();

    let mut tenant = template(Some("acme"), "1.0.0");
    tenant.display_name = "Acme Add".into();
    store.put(tenant).await.unwrap();

    let id = TemplateId::from("com.acme.add-device");
    // Tenant caller sees the tenant override.
    let seen = store.get(Some("acme"), &id, None).await.unwrap().unwrap();
    assert_eq!(seen.display_name, "Acme Add");

    // A different tenant inherits the global row.
    let other = store.get(Some("zzz"), &id, None).await.unwrap().unwrap();
    assert_eq!(other.display_name, "Global Add");

    // List for acme: tenant row hides the global of the same (id, version).
    let summaries = store
        .list(TemplateFilter {
            tenant_id: Some("acme".into()),
            category: None,
        })
        .await
        .unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].tenant_id.as_deref(), Some("acme"));
    assert_ne!(summaries[0].tenant_id.as_deref(), Some(GLOBAL_TENANT_SENTINEL));
}

#[tokio::test]
async fn run_index_lifecycle_and_open_set() {
    let pool = boot_pool().await;
    let runs = SqliteSetupRunStore::new(pool);

    let rid = RunId::new();
    let run = SetupRun {
        run_id: rid,
        template_id: TemplateId::from("com.acme.add-device"),
        template_version: SemVer::new(1, 0, 0),
        owner: "u-1".into(),
        tenant_id: Some("acme".into()),
        team: Some("hvac-ops".into()),
        status: SetupRunStatus::Running,
        progress: Progress {
            done: 0,
            total: 4,
            current_step: None,
        },
        failed_node: None,
        resumable: false,
        created_at: "2026-06-11T00:00:00Z".into(),
        finished_at: None,
    };
    runs.record(run).await.unwrap();

    // Running run is in the open set (crash recovery candidate).
    assert_eq!(runs.list_open().await.unwrap(), vec![rid]);

    // Progress projection.
    runs.update_progress(
        rid,
        Progress {
            done: 2,
            total: 4,
            current_step: Some("com.acme.notify".into()),
        },
        SetupRunStatus::Running,
    )
    .await
    .unwrap();
    let got = runs.get(rid).await.unwrap().unwrap();
    assert_eq!(got.progress.done, 2);

    // §8b: mark failed with a resume cursor → resumable stays in open set.
    runs.mark_failed(rid, Some("com.acme.notify".into()), true)
        .await
        .unwrap();
    let failed = runs.get(rid).await.unwrap().unwrap();
    assert_eq!(failed.status, SetupRunStatus::Failed);
    assert_eq!(failed.failed_node.as_deref(), Some("com.acme.notify"));
    assert!(failed.resumable);
    assert!(failed.finished_at.is_some());
    assert_eq!(runs.list_open().await.unwrap(), vec![rid]);

    // Finished (completed) → leaves the open set.
    runs.mark_finished(rid, SetupRunStatus::Completed, "2026-06-11T00:05:00Z".into())
        .await
        .unwrap();
    assert!(runs.list_open().await.unwrap().is_empty());

    // Filter by owner.
    let mine = runs
        .list(SetupRunFilter {
            owner: Some("u-1".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(mine.len(), 1);
}
