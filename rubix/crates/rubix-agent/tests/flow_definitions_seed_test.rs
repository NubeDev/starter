//! Integration coverage for the Phase D.1 `flows_definitions`
//! seed-and-load contract.
//!
//! Spins an ephemeral Postgres, applies the rubix migration
//! source, runs `flows_seed::seed_and_load` twice, and asserts:
//!
//! 1. The first call inserts one row per bundled YAML file
//!    (currently six) and the returned triple list matches the
//!    same count.
//! 2. The second call inserts zero rows (idempotency) and still
//!    returns the same triple list — the second boot is a
//!    no-op.
//! 3. Every bundled flow id is present in the live row set and
//!    each row's `revision_id` round-trips through the in-memory
//!    `FlowRevisionId`.

use rubix_agent::boot::flows_seed::{seed_and_load, SYSTEM_TENANT};
use rubix_store_postgres::FLOWS_DEFINITIONS_MIGRATION_SOURCE;
use starter_store_postgres::{migrate, testing::with_database};

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn first_boot_seeds_bundled_yamls_second_boot_is_noop() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(FLOWS_DEFINITIONS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply flows_definitions migration");

    // First boot: every bundled YAML lands as one row.
    let (triples_first, inserted_first) = seed_and_load(&pool).await.expect("first seed succeeds");
    let bundled_count = rubix_flows::bundled()
        .entries()
        .iter()
        .filter(|e| {
            e.path()
                .extension()
                .map(|x| x == "yaml" || x == "yml")
                .unwrap_or(false)
        })
        .count();
    assert_eq!(
        inserted_first, bundled_count,
        "first boot must insert one row per bundled YAML",
    );
    assert_eq!(
        triples_first.len(),
        bundled_count,
        "first-boot triples must match bundled YAML count",
    );

    let row_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM flows_definitions
          WHERE tenant_id = $1::uuid AND superseded_at IS NULL",
    )
    .bind(SYSTEM_TENANT)
    .fetch_one(pool.sqlx())
    .await
    .expect("count live rows");
    assert_eq!(row_count as usize, bundled_count);

    // Spot-check one well-known flow id is present.
    let has_flow_programmer: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM flows_definitions
          WHERE flow_id = 'com.rubix.flow-programmer'
            AND superseded_at IS NULL",
    )
    .fetch_optional(pool.sqlx())
    .await
    .expect("select flow-programmer row");
    assert!(
        has_flow_programmer.is_some(),
        "flow-programmer must land in flows_definitions on first boot",
    );

    // Second boot: idempotent — zero inserts, same load count.
    let (triples_second, inserted_second) =
        seed_and_load(&pool).await.expect("second seed succeeds");
    assert_eq!(
        inserted_second, 0,
        "second boot must insert zero rows (idempotency)",
    );
    assert_eq!(
        triples_second.len(),
        triples_first.len(),
        "second-boot load count must equal first",
    );

    // Revision ids round-trip — the in-memory FlowRevisionId for
    // each loaded triple must match the PG row's revision_id text.
    for (flow_id, revision, _body) in &triples_second {
        let pg_rev: String = sqlx::query_scalar(
            "SELECT revision_id FROM flows_definitions
              WHERE flow_id = $1 AND superseded_at IS NULL",
        )
        .bind(flow_id.to_string())
        .fetch_one(pool.sqlx())
        .await
        .expect("select revision_id");
        assert_eq!(
            pg_rev,
            revision.to_string(),
            "revision_id round-trip must be lossless for {flow_id}",
        );
    }
}
