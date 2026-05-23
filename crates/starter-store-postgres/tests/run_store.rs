//! Integration tests for [`PgRunStore`]. Twin of the `run_store_*`
//! / checkpoint / dedup cases in
//! `starter-store-sqlite/tests/flow.rs`.

#![cfg(all(feature = "flow", feature = "testing"))]

use starter_flow_spi::flow::{
    CheckpointRetention, DedupKey, FlowRevisionId, RunCheckpoint, RunId, RunOpts, RunOutcome,
    RunState, RunStore,
};
use starter_flow_spi::node::{NodeId, SlotMap, SlotRef, SlotValue};
use starter_flow_spi::Principal;
use starter_spi::auth::Role;
use starter_store_postgres::flow::{PgRunStore, FLOW_MIGRATION_SOURCE};
use starter_store_postgres::{migrate, testing::with_database, testing::ContainerGuard, Pool};

async fn boot() -> (Pool, ContainerGuard) {
    let (pool, guard) = with_database().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    (pool, guard)
}

fn fresh_principal() -> Principal {
    Principal {
        subject: "u-1".into(),
        role: Role::Reader,
        scopes: vec![],
        extra: serde_json::Value::Null,
    }
}

fn slot_ref(node: &str, slot: &str) -> SlotRef {
    SlotRef::new(NodeId::new(node).unwrap(), slot)
}

#[tokio::test]
#[ignore = "requires docker"]
async fn run_store_start_checkpoint_load_finish_roundtrip() {
    let (pool, _guard) = boot().await;
    let store = PgRunStore::new(pool);

    let run = RunId::new();
    let rev = FlowRevisionId::new();
    store
        .start(run, rev, RunOpts::default(), fresh_principal(), None)
        .await
        .unwrap();

    let writes = vec![(slot_ref("com.acme.n", "out"), SlotValue::Int(42))];
    store
        .checkpoint(run, 1, RunState::Running, &writes)
        .await
        .unwrap();
    store
        .checkpoint(run, 2, RunState::Running, &writes)
        .await
        .unwrap();

    let loaded: RunCheckpoint = store.load(run).await.unwrap().expect("checkpoint exists");
    assert_eq!(loaded.seq, 2);
    assert_eq!(loaded.state, RunState::Running);
    assert_eq!(loaded.writes.len(), 1);

    assert_eq!(store.list_open().await.unwrap(), vec![run]);

    store
        .finish(
            run,
            RunOutcome::Completed {
                output: SlotMap::default(),
            },
        )
        .await
        .unwrap();

    // After finish: no longer listed as open.
    assert!(store.list_open().await.unwrap().is_empty());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn checkpoint_retention_prunes_to_bound() {
    let (pool, _guard) = boot().await;
    let store = PgRunStore::new(pool.clone());

    let run = RunId::new();
    let mut opts = RunOpts::default();
    opts.checkpoint_retention = CheckpointRetention::Bounded(100);
    store
        .start(run, FlowRevisionId::new(), opts, fresh_principal(), None)
        .await
        .unwrap();

    let writes: Vec<(SlotRef, SlotValue)> = vec![];
    for seq in 1u64..=200 {
        store
            .checkpoint(run, seq, RunState::Running, &writes)
            .await
            .unwrap();
    }

    // Exactly 100 rows survive, min(seq) = 101.
    let (count, min_seq): (i64, i64) =
        sqlx::query_as("SELECT COUNT(*)::bigint, MIN(seq) FROM run_checkpoints WHERE run_id = $1")
            .bind(run.0.to_string())
            .fetch_one(pool.sqlx())
            .await
            .unwrap();
    assert_eq!(count, 100);
    assert_eq!(min_seq, 101);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn finish_keeps_exactly_one_final_row() {
    let (pool, _guard) = boot().await;
    let store = PgRunStore::new(pool.clone());

    let run = RunId::new();
    store
        .start(
            run,
            FlowRevisionId::new(),
            RunOpts::default(),
            fresh_principal(),
            None,
        )
        .await
        .unwrap();

    let writes: Vec<(SlotRef, SlotValue)> = vec![];
    for seq in 1u64..=5 {
        store
            .checkpoint(run, seq, RunState::Running, &writes)
            .await
            .unwrap();
    }

    store.finish(run, RunOutcome::Cancelled).await.unwrap();

    let (count, max_seq): (i64, i64) =
        sqlx::query_as("SELECT COUNT(*)::bigint, MAX(seq) FROM run_checkpoints WHERE run_id = $1")
            .bind(run.0.to_string())
            .fetch_one(pool.sqlx())
            .await
            .unwrap();
    assert_eq!(count, 1);
    assert_eq!(max_seq, 5);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn dedup_pair_unique_collision_surfaces_as_backend() {
    let (pool, _guard) = boot().await;
    let store = PgRunStore::new(pool);

    let dedup = DedupKey::new("com.acme.svc", "abc");
    let r1 = RunId::new();
    let r2 = RunId::new();
    store
        .start(
            r1,
            FlowRevisionId::new(),
            RunOpts::default(),
            fresh_principal(),
            Some(dedup.clone()),
        )
        .await
        .expect("first dedup start succeeds");

    // find_by_dedup_key locates the first run.
    let found = store
        .find_by_dedup_key(&dedup.service_name, &dedup.key)
        .await
        .unwrap();
    assert_eq!(found, Some(r1));

    // Second start with the same pair collides on the partial
    // UNIQUE index — surfaces as a typed FlowError::Backend.
    let err = store
        .start(
            r2,
            FlowRevisionId::new(),
            RunOpts::default(),
            fresh_principal(),
            Some(dedup),
        )
        .await
        .expect_err("second start collides");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("unique") || msg.contains("constraint") || msg.contains("duplicate"),
        "expected UNIQUE/constraint failure, got: {err}"
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn checkpoint_atomicity_failed_tx_preserves_prior_state() {
    // We can't crash mid-commit from a unit test, but we can
    // simulate the equivalent: open a tx, insert a bogus
    // checkpoint, then drop without commit. This locks D-F3.8
    // atomicity at the API boundary the propagator sees.
    let (pool, _guard) = boot().await;
    let store = PgRunStore::new(pool.clone());

    let run = RunId::new();
    store
        .start(
            run,
            FlowRevisionId::new(),
            RunOpts::default(),
            fresh_principal(),
            None,
        )
        .await
        .unwrap();

    let writes: Vec<(SlotRef, SlotValue)> = vec![];
    store
        .checkpoint(run, 1, RunState::Running, &writes)
        .await
        .unwrap();

    {
        let mut tx = pool.sqlx().begin().await.unwrap();
        sqlx::query(
            "INSERT INTO run_checkpoints (run_id, seq, run_state_json, slot_writes_json) \
             VALUES ($1, 99, '\"running\"'::jsonb, '[]'::jsonb)",
        )
        .bind(run.0.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
        // No commit — drop the tx, which rolls back.
        drop(tx);
    }

    // load() still returns seq=1; the rolled-back insert isn't
    // visible.
    let loaded = store.load(run).await.unwrap().unwrap();
    assert_eq!(loaded.seq, 1);
}
