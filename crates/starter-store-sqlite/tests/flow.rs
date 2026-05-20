//! End-to-end tests for the SQLite store impls of the
//! `starter-flow-spi` persistence seams. Covers the four
//! correctness invariants WORKFLOW.md stage 4 names — atomicity,
//! retention pruning, finish keep-final-row, dedup uniqueness —
//! plus a smoke test that the connection-init pragmas applied to
//! every pool connection.

#![cfg(all(feature = "flow", feature = "testing"))]

use sqlx::Row;
use starter_flow_spi::flow::{
    CheckpointRetention, DedupKey, FlowId, FlowRevision, FlowRevisionId, FlowStore, RunCheckpoint,
    RunId, RunOpts, RunOutcome, RunState, RunStore, SessionId, SessionRecord, SessionStore,
};
use starter_flow_spi::node::{SlotMap, SlotRef, SlotValue};
use starter_flow_spi::Principal;
use starter_spi::auth::Role;
use starter_store_sqlite::flow::{
    SqliteFlowStore, SqliteRunStore, SqliteSessionStore, FLOW_MIGRATION_SOURCE,
};
use starter_store_sqlite::{migrate, testing::ephemeral, Pool};

async fn boot_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    pool
}

fn fresh_principal() -> Principal {
    Principal {
        subject: "u-1".into(),
        role: Role::Reader,
        scopes: vec![],
        extra: serde_json::Value::Null,
    }
}

fn fresh_flow_id() -> FlowId {
    FlowId::new("com.acme.test.flow").unwrap()
}

fn slot_ref(node: &str, slot: &str) -> SlotRef {
    SlotRef::new(starter_flow_spi::node::NodeId::new(node).unwrap(), slot)
}

#[tokio::test]
async fn flow_store_put_load_head_revisions() {
    let pool = boot_pool().await;
    let store = SqliteFlowStore::new(pool);

    let fid = fresh_flow_id();
    let rev_a = FlowRevisionId::new();
    let rev_b = FlowRevisionId::new();

    store
        .put(FlowRevision::new(
            fid.clone(),
            rev_a,
            serde_json::json!({"v": 1}),
        ))
        .await
        .unwrap();
    store
        .put(FlowRevision::new(
            fid.clone(),
            rev_b,
            serde_json::json!({"v": 2}),
        ))
        .await
        .unwrap();

    // Head reflects the most recently put revision.
    assert_eq!(store.head(fid.clone()).await.unwrap(), Some(rev_b));

    // load(None) resolves to head.
    let loaded = store.load(fid.clone(), None).await.unwrap();
    assert_eq!(loaded.revision_id, rev_b);
    assert_eq!(loaded.body, serde_json::json!({"v": 2}));

    // Explicit load of the older revision still works (immutable).
    let loaded_a = store.load(fid.clone(), Some(rev_a)).await.unwrap();
    assert_eq!(loaded_a.body, serde_json::json!({"v": 1}));

    // Both revisions listed; ordering is best-effort
    // (created_at has 1s resolution, so equal-timestamp ties
    // break by revision_id DESC).
    let revs = store.revisions(fid.clone()).await.unwrap();
    assert_eq!(revs.len(), 2);
    assert!(revs.contains(&rev_a) && revs.contains(&rev_b));

    // Flow shows up in list().
    let flows = store.list().await.unwrap();
    assert_eq!(flows, vec![fid]);
}

#[tokio::test]
async fn run_store_start_checkpoint_load_finish_roundtrip() {
    let pool = boot_pool().await;
    let store = SqliteRunStore::new(pool);

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
async fn checkpoint_retention_prunes_to_bound() {
    let pool = boot_pool().await;
    let store = SqliteRunStore::new(pool.clone());

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
        sqlx::query_as("SELECT COUNT(*), MIN(seq) FROM run_checkpoints WHERE run_id = ?1")
            .bind(run.0.to_string())
            .fetch_one(pool.sqlx())
            .await
            .unwrap();
    assert_eq!(count, 100);
    assert_eq!(min_seq, 101);
}

#[tokio::test]
async fn finish_keeps_exactly_one_final_row() {
    let pool = boot_pool().await;
    let store = SqliteRunStore::new(pool.clone());

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
        sqlx::query_as("SELECT COUNT(*), MAX(seq) FROM run_checkpoints WHERE run_id = ?1")
            .bind(run.0.to_string())
            .fetch_one(pool.sqlx())
            .await
            .unwrap();
    assert_eq!(count, 1);
    assert_eq!(max_seq, 5);
}

#[tokio::test]
async fn dedup_pair_unique_collision_surfaces_as_backend() {
    let pool = boot_pool().await;
    let store = SqliteRunStore::new(pool);

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
        msg.contains("unique") || msg.contains("constraint"),
        "expected UNIQUE/constraint failure, got: {err}"
    );
}

#[tokio::test]
async fn checkpoint_atomicity_failed_tx_preserves_prior_state() {
    // We can't crash mid-commit from a unit test, but we can
    // simulate the equivalent: abort a transaction that's
    // half-built, then verify the prior checkpoint is still the
    // latest visible state. This locks the D-F3.8 atomicity
    // invariant at the API boundary the propagator sees.
    let pool = boot_pool().await;
    let store = SqliteRunStore::new(pool.clone());

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

    // Open a manual tx, insert a bogus checkpoint, then ROLLBACK.
    {
        let mut tx = pool.sqlx().begin().await.unwrap();
        sqlx::query(
            "INSERT INTO run_checkpoints (run_id, seq, run_state_json, slot_writes_json) \
             VALUES (?1, 99, '\"running\"', '[]')",
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

#[tokio::test]
async fn wal_pragmas_applied_on_file_backed_pool() {
    // Use a file-backed sqlite DB so `PRAGMA journal_mode` is
    // not silently downgraded to `memory`.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}?mode=rwc", tmp.path().display());
    let pool = starter_store_sqlite::pool::connect(&url)
        .await
        .expect("connect");
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .unwrap();

    let row = sqlx::query("PRAGMA journal_mode")
        .fetch_one(pool.sqlx())
        .await
        .unwrap();
    let mode: String = row.get(0);
    assert_eq!(mode.to_lowercase(), "wal", "journal_mode={mode}");

    let row = sqlx::query("PRAGMA synchronous")
        .fetch_one(pool.sqlx())
        .await
        .unwrap();
    let synchronous: i64 = row.get(0);
    assert_eq!(synchronous, 1, "synchronous=NORMAL expected (1)");

    let row = sqlx::query("PRAGMA foreign_keys")
        .fetch_one(pool.sqlx())
        .await
        .unwrap();
    let fk: i64 = row.get(0);
    assert_eq!(fk, 1, "foreign_keys=ON expected (1)");

    let row = sqlx::query("PRAGMA busy_timeout")
        .fetch_one(pool.sqlx())
        .await
        .unwrap();
    let bt: i64 = row.get(0);
    assert_eq!(bt, 5000, "busy_timeout=5000 expected");
}

#[tokio::test]
async fn session_store_put_get_list_by_principal() {
    let pool = boot_pool().await;
    let store = SqliteSessionStore::new(pool);

    let p1 = fresh_principal();
    let mut p2 = fresh_principal();
    p2.subject = "u-2".into();

    let s1 = SessionId::new();
    let s2 = SessionId::new();
    let s3 = SessionId::new();
    store
        .put(
            s1,
            SessionRecord::new(s1, p1.clone(), serde_json::json!({"k": 1})),
        )
        .await
        .unwrap();
    store
        .put(
            s2,
            SessionRecord::new(s2, p1.clone(), serde_json::json!({"k": 2})),
        )
        .await
        .unwrap();
    store
        .put(
            s3,
            SessionRecord::new(s3, p2.clone(), serde_json::json!({"k": 3})),
        )
        .await
        .unwrap();

    // Upsert: re-put s1 with new body.
    store
        .put(
            s1,
            SessionRecord::new(s1, p1.clone(), serde_json::json!({"k": 11})),
        )
        .await
        .unwrap();
    let loaded = store.get(s1).await.unwrap().unwrap();
    assert_eq!(loaded.body, serde_json::json!({"k": 11}));

    // list(p1) returns s1+s2 (order = created_at).
    let mut listed = store.list(p1).await.unwrap();
    listed.sort_by_key(|s| s.0);
    let mut expected = vec![s1, s2];
    expected.sort_by_key(|s| s.0);
    assert_eq!(listed, expected);

    // Missing session => None, not a backend error.
    assert!(store.get(SessionId::new()).await.unwrap().is_none());
}
