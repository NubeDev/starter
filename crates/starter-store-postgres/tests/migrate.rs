//! End-to-end migration test against a real ephemeral Postgres
//! container. Mirrors `starter-store-sqlite/tests/migrate.rs` so the
//! two backends are exercised against the same invariants.
//!
//! **Marked `#[ignore]`** because it requires Docker on the host —
//! CI runs it explicitly via `cargo test -p starter-store-postgres
//! --features testing -- --ignored`. Local dev: same invocation, or
//! set `STARTER_RUN_DOCKER_TESTS=1` and drop the flag in a wrapper.

#![cfg(feature = "testing")]

use sqlx::Row;
use starter_store_postgres::{migrate, migrate::MigrationSource, testing::with_database};

static STARTER: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/starter");
static APP: sqlx::migrate::Migrator = sqlx::migrate!("./tests/fixtures/app");

#[tokio::test]
#[ignore = "requires docker"]
async fn two_sources_apply_without_colliding() {
    let (pool, _guard) = with_database().await;

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

    // The migrations actually created their tables.
    let starter_schema: String = sqlx::query("SELECT value FROM starter_meta WHERE key = 'schema'")
        .fetch_one(pool.sqlx())
        .await
        .unwrap()
        .get(0);
    assert_eq!(starter_schema, "starter-v1");

    let widget_count: i64 = sqlx::query("SELECT COUNT(*) FROM app_widgets")
        .fetch_one(pool.sqlx())
        .await
        .unwrap()
        .get(0);
    assert_eq!(widget_count, 0);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn rerun_is_a_noop() {
    let (pool, _guard) = with_database().await;

    let plan = || {
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
    };

    plan().await.expect("first run");
    plan().await.expect("second run is a no-op");

    // Still exactly one row in each per-source table.
    let rows: i64 = sqlx::query("SELECT COUNT(*) FROM _sqlx_migrations_starter")
        .fetch_one(pool.sqlx())
        .await
        .unwrap()
        .get(0);
    assert_eq!(rows, 1);
}
