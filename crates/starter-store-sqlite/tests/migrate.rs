//! End-to-end test for the namespaced migration runner against a real
//! in-memory SQLite database. Locks the no-collision invariant
//! between two sources that both start at version 1.

#![cfg(feature = "testing")]

use sqlx::Row;
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral};

// Two independent sources, both starting at version 1 — exactly the
// case namespacing exists to support.
static STARTER: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/starter");
static APP: sqlx::migrate::Migrator = sqlx::migrate!("./tests/fixtures/app");

#[tokio::test]
async fn two_sources_apply_without_colliding() {
    let pool = ephemeral().await;

    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter",
            migrator: &STARTER,
        })
        .with_source(MigrationSource {
            name: "app",
            migrator: &APP,
        })
        .run()
        .await
        .expect("migrations apply");

    // Both per-source tables exist, populated with version=1.
    let starter_version: i64 =
        sqlx::query("SELECT version FROM _sqlx_migrations_starter WHERE version = 1")
            .fetch_one(pool.sqlx())
            .await
            .unwrap()
            .get(0);
    assert_eq!(starter_version, 1);

    let app_version: i64 =
        sqlx::query("SELECT version FROM _sqlx_migrations_app WHERE version = 1")
            .fetch_one(pool.sqlx())
            .await
            .unwrap()
            .get(0);
    assert_eq!(app_version, 1);

    // The actual schemas the migrations created are both reachable.
    let starter_schema: String = sqlx::query("SELECT value FROM starter_meta WHERE key = 'schema'")
        .fetch_one(pool.sqlx())
        .await
        .unwrap()
        .get(0);
    assert_eq!(starter_schema, "starter-v1");

    let app_row_count: i64 = sqlx::query("SELECT count(*) FROM app_widgets")
        .fetch_one(pool.sqlx())
        .await
        .unwrap()
        .get(0);
    assert_eq!(app_row_count, 0);
}

#[tokio::test]
async fn rerun_is_a_noop() {
    let pool = ephemeral().await;
    let plan = || {
        migrate(&pool).with_source(MigrationSource {
            name: "starter",
            migrator: &STARTER,
        })
    };
    plan().run().await.expect("first run");
    plan().run().await.expect("second run is a no-op");

    let count: i64 = sqlx::query("SELECT count(*) FROM _sqlx_migrations_starter")
        .fetch_one(pool.sqlx())
        .await
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}

#[tokio::test]
async fn invalid_source_name_is_rejected() {
    let pool = ephemeral().await;
    let err = migrate(&pool)
        .with_source(MigrationSource {
            name: "bad name!",
            migrator: &STARTER,
        })
        .run()
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid migration source name"), "{msg}");
}
