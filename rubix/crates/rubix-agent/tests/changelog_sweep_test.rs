//! Integration coverage for the `starter_changes` retention sweep
//! and the rubix-side `changelog_kind_policy` seed.
//!
//! Spins an ephemeral Postgres, runs the full rubix migration chain
//! (which includes both the `changelog` source that creates
//! `changelog_kind_policy` and the rubix `changelog_policy` source
//! that seeds the audit-floor rows), then asserts:
//!
//! 1. The seed migration lands `user` and `team` as audit-floor
//!    rows (`max_age_days = NULL`).
//! 2. `sweep_changelog_once` against that state leaves
//!    audit-floored rows untouched even when they are arbitrarily
//!    old — the floor is the load-bearing security contract.
//! 3. A separate kind with an opt-in finite curve does get pruned,
//!    proving the sweep is wired and the per-kind isolation holds
//!    end-to-end through the boot path.
//!
//! See `rubix/docs/proposal/audit-log.md` for the rationale.

use chrono::{Duration, Utc};
use rubix_agent::boot::sweep_changelog_once;
use rubix_store_postgres::CHANGELOG_POLICY_MIGRATION_SOURCE;
use starter_changelog_postgres::migration_source as changelog_source;
use starter_store_postgres::{migrate, testing::with_database, Pool};

async fn insert_change_at_age(pool: &Pool, kind: &str, id: &str, days_old: i64) {
    let at = Utc::now() - Duration::days(days_old);
    sqlx::query(
        "INSERT INTO starter_changes \
            (id, at, actor_kind, resource_kind, resource_id, op, group_id) \
         VALUES ($1, $2, 'system', $3, 'r1', 'update', $1)",
    )
    .bind(id)
    .bind(at)
    .bind(kind)
    .execute(pool.sqlx())
    .await
    .expect("insert");
}

async fn count(pool: &Pool, kind: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM starter_changes WHERE resource_kind = $1")
        .bind(kind)
        .fetch_one(pool.sqlx())
        .await
        .expect("count")
}

async fn run_migrations(pool: &Pool) {
    // Mirror the source order in `rubix-agent::boot::migrations`:
    // the upstream `changelog` source provisions the
    // `changelog_kind_policy` table; the rubix-owned
    // `changelog_policy` source seeds it. Order matters — the seed
    // INSERTs would fail against a missing table.
    migrate(pool)
        .with_source(changelog_source())
        .with_source(CHANGELOG_POLICY_MIGRATION_SOURCE)
        .run()
        .await
        .expect("migrations");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn seed_lands_user_and_team_as_audit_floor() {
    let (pool, _guard) = with_database().await;
    run_migrations(&pool).await;

    let floor: Vec<(String, Option<i32>)> = sqlx::query_as(
        "SELECT resource_kind, max_age_days FROM changelog_kind_policy ORDER BY resource_kind",
    )
    .fetch_all(pool.sqlx())
    .await
    .expect("read policy");

    assert_eq!(
        floor,
        vec![
            ("team".to_string(), None),
            ("user".to_string(), None),
        ],
        "rubix seed must pin `user` and `team` to NULL (keep forever)",
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn sweep_respects_audit_floor_and_prunes_opt_in_kind() {
    let (pool, _guard) = with_database().await;
    run_migrations(&pool).await;

    // Opt `flow_def` into a 30-day curve so the sweep has something
    // to delete; `user`/`team` remain on the seeded NULL floor.
    sqlx::query(
        "INSERT INTO changelog_kind_policy (resource_kind, max_age_days) VALUES ('flow_def', 30)",
    )
    .execute(pool.sqlx())
    .await
    .expect("seed flow_def policy");

    // Ancient rows across all three kinds — only `flow_def` should
    // be touched.
    insert_change_at_age(&pool, "user", "u-old", 5_000).await;
    insert_change_at_age(&pool, "team", "t-old", 5_000).await;
    insert_change_at_age(&pool, "flow_def", "f-old", 5_000).await;
    insert_change_at_age(&pool, "flow_def", "f-fresh", 5).await;

    let report = sweep_changelog_once(&pool).await.expect("sweep");
    // `user` + `team` are on NULL curves and must not appear in
    // the per-kind report at all (the helper filters them out
    // before issuing any DELETE).
    let kinds: Vec<&str> = report.per_kind.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        kinds,
        vec!["flow_def"],
        "audit-floor kinds must not be touched by the sweep",
    );
    assert_eq!(report.total_deleted(), 1, "only the 5_000-day flow_def row");

    assert_eq!(count(&pool, "user").await, 1, "user audit floor preserved");
    assert_eq!(count(&pool, "team").await, 1, "team audit floor preserved");
    assert_eq!(count(&pool, "flow_def").await, 1, "fresh flow_def kept");
}
