//! Docker-backed tests for `starter_changelog_postgres::policy`.
//!
//! Exercises the per-kind retention sweep added in
//! migration `0004_changelog_kind_policy.sql`.
//!
//! **`#[ignore]`** by default — requires Docker. Run with
//! `cargo test -p starter-changelog-postgres --test policy -- --ignored`.
//! Wired into CI under the `undo-postgres` job.

use chrono::{Duration, Utc};
use starter_changelog_postgres::{apply_policy, migration_source};
use starter_store_postgres::{migrate, testing::with_database, Pool};

async fn setup() -> (Pool, starter_store_postgres::testing::ContainerGuard) {
    let (pool, guard) = with_database().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("changelog migrations");
    (pool, guard)
}

/// Insert one `starter_changes` row at the given age. Uses raw SQL
/// rather than `PgChangeRecorder` so the test can backdate the `at`
/// column — the recorder always stamps `NOW()`.
async fn insert_at_age(pool: &Pool, kind: &str, days_old: i64) {
    let at = Utc::now() - Duration::days(days_old);
    sqlx::query(
        "INSERT INTO starter_changes \
            (id, at, actor_kind, resource_kind, resource_id, op, group_id) \
         VALUES ($1, $2, 'system', $3, 'r1', 'update', $1)",
    )
    .bind(format!("c-{kind}-{days_old}"))
    .bind(at)
    .bind(kind)
    .execute(pool.sqlx())
    .await
    .expect("insert");
}

async fn row_count(pool: &Pool, kind: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM starter_changes WHERE resource_kind = $1")
        .bind(kind)
        .fetch_one(pool.sqlx())
        .await
        .expect("count")
}

#[tokio::test]
#[ignore = "requires docker"]
async fn no_policy_rows_means_no_deletes() {
    // Empty `changelog_kind_policy` is the today-baseline: implicit
    // unbounded retention. The helper must leave every row intact.
    let (pool, _guard) = setup().await;

    insert_at_age(&pool, "user", 30).await;
    insert_at_age(&pool, "user", 400).await;
    insert_at_age(&pool, "team", 999).await;

    let report = apply_policy(&pool).await.expect("apply");
    assert_eq!(report.total_deleted(), 0, "no opt-in → no deletes");
    assert!(report.per_kind.is_empty(), "no kinds reported");

    assert_eq!(row_count(&pool, "user").await, 2);
    assert_eq!(row_count(&pool, "team").await, 1);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn null_max_age_pins_kind_to_unbounded() {
    // A NULL `max_age_days` row is the "audit floor" marker — the
    // operator has declared this kind exempt from any future sweep.
    // The helper must skip it even though a row exists.
    let (pool, _guard) = setup().await;

    sqlx::query("INSERT INTO changelog_kind_policy (resource_kind, max_age_days) VALUES ('user', NULL)")
        .execute(pool.sqlx())
        .await
        .expect("seed");

    insert_at_age(&pool, "user", 30).await;
    insert_at_age(&pool, "user", 5_000).await;

    let report = apply_policy(&pool).await.expect("apply");
    assert_eq!(report.total_deleted(), 0, "NULL curve = keep forever");
    assert!(
        report.per_kind.is_empty(),
        "NULL rows are filtered out of the work set"
    );
    assert_eq!(row_count(&pool, "user").await, 2);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn finite_max_age_deletes_older_rows() {
    let (pool, _guard) = setup().await;

    sqlx::query("INSERT INTO changelog_kind_policy (resource_kind, max_age_days) VALUES ('flow_def', 30)")
        .execute(pool.sqlx())
        .await
        .expect("seed");

    insert_at_age(&pool, "flow_def", 5).await; // keep
    insert_at_age(&pool, "flow_def", 29).await; // keep (boundary)
    insert_at_age(&pool, "flow_def", 90).await; // drop
    insert_at_age(&pool, "flow_def", 365).await; // drop

    let report = apply_policy(&pool).await.expect("apply");
    assert_eq!(report.per_kind, vec![("flow_def".to_string(), 2)]);
    assert_eq!(row_count(&pool, "flow_def").await, 2);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn policy_is_per_kind_does_not_touch_unlisted_kinds() {
    // Pin the contract that operator-driven retention on one kind
    // cannot collateral-damage rows from a different kind that
    // happen to be old. Critical for the audit-floor story: a sweep
    // for `flow_def` must never touch `user`.
    let (pool, _guard) = setup().await;

    sqlx::query("INSERT INTO changelog_kind_policy (resource_kind, max_age_days) VALUES ('flow_def', 30)")
        .execute(pool.sqlx())
        .await
        .expect("seed");

    insert_at_age(&pool, "user", 5_000).await;
    insert_at_age(&pool, "flow_def", 5_000).await;

    let report = apply_policy(&pool).await.expect("apply");
    assert_eq!(report.per_kind, vec![("flow_def".to_string(), 1)]);
    assert_eq!(row_count(&pool, "user").await, 1, "user untouched");
    assert_eq!(row_count(&pool, "flow_def").await, 0, "flow_def swept");
}
