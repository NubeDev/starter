//! Integration coverage for the `undo_snapshots` retention sweep.
//!
//! Spins an ephemeral Postgres, applies the rubix migration source,
//! seeds 100 snapshots for a single
//! `(tenant_id, resource_kind, resource_id)` triple, runs the sweep
//! with the default `UndoConfig`, and asserts the table is pruned
//! to ≤ 50 rows — proving the per-resource `max_rows_per_resource`
//! limit is enforced. A second pass with a tight `max_age_days`
//! exercises the age-based DELETE path.

use rubix_agent::boot::{sweep_undo_once, UndoConfig};
use rubix_store_postgres::UNDO_SNAPSHOTS_MIGRATION_SOURCE;
use starter_store_postgres::{migrate, testing::with_database};

const TENANT: &str = "00000000-0000-0000-0000-000000000000";
const ACTOR: &str = "00000000-0000-0000-0000-000000000001";

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn sweep_prunes_per_resource_to_default_cap() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(UNDO_SNAPSHOTS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply undo migration");

    // Seed 100 rows under one (tenant, kind, id) bucket. The
    // `created_at` stride keeps row_number ordering deterministic;
    // ULID-as-TEXT ids are unique by simple counter padding.
    for i in 0..100u32 {
        let id = format!("{i:026}");
        sqlx::query(
            "INSERT INTO undo_snapshots
                (id, tenant_id, actor_id, resource_kind, resource_id,
                 snapshot_jsonb, created_at)
             VALUES ($1, $2::uuid, $3::uuid, 'user', 'u-1', '{}'::jsonb,
                     NOW() - ($4 || ' seconds')::interval)",
        )
        .bind(&id)
        .bind(TENANT)
        .bind(ACTOR)
        .bind((100 - i) as i32)
        .execute(pool.sqlx())
        .await
        .expect("seed row");
    }

    let cfg = UndoConfig::default();
    let deleted = sweep_undo_once(&pool, &cfg).await.expect("sweep succeeds");
    assert!(
        deleted >= 50,
        "expected ≥ 50 deleted rows under the default 50-row cap, got {deleted}",
    );

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM undo_snapshots")
        .fetch_one(pool.sqlx())
        .await
        .expect("count remaining");
    assert!(
        remaining <= cfg.max_rows_per_resource as i64,
        "sweep left {remaining} rows; cap is {}",
        cfg.max_rows_per_resource,
    );

    // Age-based path: shrink max_age_days to 0 and confirm every
    // remaining row is wiped (none of the seeded rows are < NOW()).
    let aged_cfg = UndoConfig {
        max_rows_per_resource: 1_000,
        max_age_days: 0,
    };
    let aged_deleted = sweep_undo_once(&pool, &aged_cfg)
        .await
        .expect("age sweep succeeds");
    assert_eq!(aged_deleted as i64, remaining);
    let final_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM undo_snapshots")
        .fetch_one(pool.sqlx())
        .await
        .expect("final count");
    assert_eq!(final_count, 0, "age-zero sweep clears the table");
}
