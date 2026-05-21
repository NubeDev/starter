//! Integration tests for [`PostgresAgentSessionStore`]
//! (MEMORY.md Phase M-B). Twin of
//! `starter-store-sqlite/tests/agent_sessions.rs` — identical
//! assertions, identical invariants, identical structure.
//!
//! Marked `#[ignore]` because it requires Docker on the host (same
//! pattern as `tests/migrate.rs` and `tests/skills.rs`). CI runs it
//! explicitly via
//! `cargo test -p starter-store-postgres --features
//!  "testing agent-session" -- --ignored`.

#![cfg(all(feature = "agent-session", feature = "testing"))]

use starter_flow_spi::agent_session::{
    AgentSessionError, AgentSessionId, AgentSessionStore, ArtifactWrite, PutArtifactError,
    RetentionPolicy, TurnInput, TurnRole, ARTIFACT_VALUE_CAP_BYTES, TURN_CONTENT_CAP_BYTES,
};
use starter_store_postgres::agent_session::{
    PostgresAgentSessionStore, AGENT_SESSION_MIGRATION_SOURCE,
};
use starter_store_postgres::{migrate, testing::with_database, Pool};

async fn boot() -> (Pool, starter_store_postgres::testing::ContainerGuard) {
    let (pool, guard) = with_database().await;
    migrate(&pool)
        .with_source(AGENT_SESSION_MIGRATION_SOURCE)
        .run()
        .await
        .expect("agent-session migrations apply");
    (pool, guard)
}

async fn fresh_session(store: &PostgresAgentSessionStore) -> AgentSessionId {
    let id = AgentSessionId::new();
    store
        .create(id, "page-builder", "system", serde_json::json!({}))
        .await
        .expect("create");
    id
}

#[tokio::test]
#[ignore = "requires docker"]
async fn create_get_roundtrip() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = AgentSessionId::new();
    store
        .create(
            id,
            "chat",
            "user-7",
            serde_json::json!({"provider": "claude"}),
        )
        .await
        .unwrap();
    let got = store.get(id).await.unwrap().expect("session");
    assert_eq!(got.id, id);
    assert_eq!(got.kind, "chat");
    assert_eq!(got.owner, "user-7");
    assert_eq!(got.metadata, serde_json::json!({"provider": "claude"}));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn append_assigns_monotonic_seq_per_session() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    let r1 = store
        .append_turn_with_artifacts(
            id,
            TurnInput::new(TurnRole::User, serde_json::json!({"text": "hi"})),
            &[],
        )
        .await
        .unwrap();
    assert_eq!(r1.seq, 1);

    let r2 = store
        .append_turn_with_artifacts(
            id,
            TurnInput::new(TurnRole::Assistant, serde_json::json!({"text": "hello"})),
            &[],
        )
        .await
        .unwrap();
    assert_eq!(r2.seq, 2);

    let turns = store.list_turns(id, None, None).await.unwrap();
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].seq, 1);
    assert_eq!(turns[1].seq, 2);
    assert!(matches!(turns[0].role, TurnRole::User));
    assert!(matches!(turns[1].role, TurnRole::Assistant));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn artifact_versions_are_monotonic_per_key() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    let r1 = store
        .append_turn_with_artifacts(
            id,
            TurnInput::new(TurnRole::User, serde_json::json!({})),
            &[ArtifactWrite::new("tree", serde_json::json!({"v": 1}))],
        )
        .await
        .unwrap();
    assert_eq!(r1.artifact_versions, vec![1]);

    let r2 = store
        .append_turn_with_artifacts(
            id,
            TurnInput::new(TurnRole::User, serde_json::json!({})),
            &[
                ArtifactWrite::new("tree", serde_json::json!({"v": 2})).with_parent(1),
                ArtifactWrite::new("draft", serde_json::json!({"d": 1})),
            ],
        )
        .await
        .unwrap();
    assert_eq!(r2.artifact_versions, vec![2, 1]);

    let latest_tree = store.latest_artifact(id, "tree").await.unwrap().unwrap();
    assert_eq!(latest_tree.version, 2);
    assert_eq!(latest_tree.parent_version, Some(1));
    assert_eq!(latest_tree.value, serde_json::json!({"v": 2}));
    assert_eq!(latest_tree.produced_by_seq, Some(2));

    let v1 = store.artifact_at(id, "tree", 1).await.unwrap().unwrap();
    assert_eq!(v1.version, 1);
    assert_eq!(v1.value, serde_json::json!({"v": 1}));

    let versions = store.list_artifact_versions(id, "tree").await.unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 2);
    assert_eq!(versions[1].version, 1);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn append_to_missing_session_errors() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let bogus = AgentSessionId::new();
    let err = store
        .append_turn_with_artifacts(
            bogus,
            TurnInput::new(TurnRole::User, serde_json::json!({})),
            &[],
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AgentSessionError::SessionNotFound(_)));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn turn_too_large_is_rejected_without_writing() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    let big = "x".repeat(TURN_CONTENT_CAP_BYTES + 16);
    let err = store
        .append_turn_with_artifacts(
            id,
            TurnInput::new(TurnRole::User, serde_json::json!({ "text": big })),
            &[],
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AgentSessionError::TurnTooLarge { .. }));

    let turns = store.list_turns(id, None, None).await.unwrap();
    assert!(turns.is_empty());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn artifact_too_large_is_rejected_without_writing() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    let big = "x".repeat(ARTIFACT_VALUE_CAP_BYTES + 16);
    let err = store
        .append_turn_with_artifacts(
            id,
            TurnInput::new(TurnRole::Assistant, serde_json::json!({})),
            &[ArtifactWrite::new("tree", serde_json::json!({ "blob": big }))],
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AgentSessionError::ArtifactTooLarge { .. }));

    let turns = store.list_turns(id, None, None).await.unwrap();
    assert!(turns.is_empty());
    assert!(store.latest_artifact(id, "tree").await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn put_artifact_direct_assigns_version_and_lineage() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    let v1 = store
        .put_artifact_direct(id, "tree", serde_json::json!({"v": 1}), None)
        .await
        .unwrap();
    assert_eq!(v1, 1);

    let v2 = store
        .put_artifact_direct(id, "tree", serde_json::json!({"v": 2}), None)
        .await
        .unwrap();
    assert_eq!(v2, 2);

    let row = store.latest_artifact(id, "tree").await.unwrap().unwrap();
    assert_eq!(row.version, 2);
    assert_eq!(row.parent_version, Some(1));
    assert!(row.produced_by_seq.is_none());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn put_artifact_direct_optimistic_conflict_returns_current() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    store
        .put_artifact_direct(id, "tree", serde_json::json!({"v": 1}), None)
        .await
        .unwrap();
    store
        .put_artifact_direct(id, "tree", serde_json::json!({"v": 2}), Some(1))
        .await
        .unwrap();

    let err = store
        .put_artifact_direct(id, "tree", serde_json::json!({"v": 99}), Some(1))
        .await
        .unwrap_err();
    match err {
        PutArtifactError::Conflict { current } => assert_eq!(current, 2),
        other => panic!("expected Conflict, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn delete_cascades_through_turns_and_artifacts() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    store
        .append_turn_with_artifacts(
            id,
            TurnInput::new(TurnRole::User, serde_json::json!({})),
            &[ArtifactWrite::new("tree", serde_json::json!({}))],
        )
        .await
        .unwrap();

    store.delete(id).await.unwrap();
    assert!(store.get(id).await.unwrap().is_none());
    assert!(store.latest_artifact(id, "tree").await.unwrap().is_none());
    let turns = store.list_turns(id, None, None).await.unwrap();
    assert!(turns.is_empty());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn list_turns_paginates_by_since_seq_and_limit() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    for i in 0..5 {
        store
            .append_turn_with_artifacts(
                id,
                TurnInput::new(TurnRole::User, serde_json::json!({ "i": i })),
                &[],
            )
            .await
            .unwrap();
    }

    let page1 = store.list_turns(id, None, Some(2)).await.unwrap();
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].seq, 1);
    assert_eq!(page1[1].seq, 2);

    let page2 = store
        .list_turns(id, Some(page1.last().unwrap().seq), Some(2))
        .await
        .unwrap();
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].seq, 3);
    assert_eq!(page2[1].seq, 4);

    let tail = store.list_turns(id, Some(4), None).await.unwrap();
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].seq, 5);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tokens_in_and_out_roundtrip_when_set() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    let mut input = TurnInput::new(TurnRole::Assistant, serde_json::json!({"text": "ok"}));
    input.tokens_in = Some(123);
    input.tokens_out = Some(45);
    store
        .append_turn_with_artifacts(id, input, &[])
        .await
        .unwrap();

    let turns = store.list_turns(id, None, None).await.unwrap();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].tokens_in, Some(123));
    assert_eq!(turns[0].tokens_out, Some(45));
}

// ---------------------------------------------------------------------
// Retention (MEMORY.md §M9 / Phase M-E)
// ---------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires docker"]
async fn retention_keep_forever_is_noop() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;
    store
        .append_turn_with_artifacts(
            id,
            TurnInput::new(TurnRole::User, serde_json::json!({"text": "hi"})),
            &[],
        )
        .await
        .unwrap();

    let report = store
        .sweep_retention(
            "page-builder",
            &RetentionPolicy::KeepForever,
            chrono::Utc::now() + chrono::Duration::days(365),
        )
        .await
        .unwrap();
    assert_eq!(report.sessions_deleted, 0);
    assert_eq!(report.turns_deleted, 0);
    assert_eq!(report.artifacts_deleted, 0);
    assert!(store.get(id).await.unwrap().is_some());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn retention_delete_after_cascades_turns_and_artifacts() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;
    store
        .append_turn_with_artifacts(
            id,
            TurnInput::new(TurnRole::Assistant, serde_json::json!({"text": "out"})),
            &[ArtifactWrite::new("tree", serde_json::json!({"root": {}}))],
        )
        .await
        .unwrap();

    let untouched = store
        .sweep_retention(
            "chat",
            &RetentionPolicy::DeleteAfter {
                ttl: chrono::Duration::seconds(0),
            },
            chrono::Utc::now() + chrono::Duration::days(365),
        )
        .await
        .unwrap();
    assert_eq!(untouched.sessions_deleted, 0);
    assert!(store.get(id).await.unwrap().is_some());

    let report = store
        .sweep_retention(
            "page-builder",
            &RetentionPolicy::DeleteAfter {
                ttl: chrono::Duration::days(1),
            },
            chrono::Utc::now() + chrono::Duration::days(365),
        )
        .await
        .unwrap();
    assert_eq!(report.sessions_deleted, 1);
    assert_eq!(report.turns_deleted, 1);
    assert_eq!(report.artifacts_deleted, 1);

    assert!(store.get(id).await.unwrap().is_none());
    assert!(store.latest_artifact(id, "tree").await.unwrap().is_none());
    let turns = store.list_turns(id, None, None).await.unwrap();
    assert!(turns.is_empty());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn retention_delete_turns_after_keeps_latest_artifact() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;

    for i in 1..=2 {
        store
            .append_turn_with_artifacts(
                id,
                TurnInput::new(TurnRole::Assistant, serde_json::json!({"i": i})),
                &[ArtifactWrite::new("tree", serde_json::json!({"v": i}))],
            )
            .await
            .unwrap();
    }
    let versions_before = store.list_artifact_versions(id, "tree").await.unwrap();
    assert_eq!(versions_before.len(), 2);

    let report = store
        .sweep_retention(
            "page-builder",
            &RetentionPolicy::DeleteTurnsAfter {
                ttl: chrono::Duration::days(1),
                keep_latest_artifact: true,
            },
            chrono::Utc::now() + chrono::Duration::days(365),
        )
        .await
        .unwrap();
    assert_eq!(report.sessions_deleted, 0);
    assert_eq!(report.turns_deleted, 2);
    assert_eq!(report.artifacts_deleted, 1);

    assert!(store.get(id).await.unwrap().is_some());
    let versions_after = store.list_artifact_versions(id, "tree").await.unwrap();
    assert_eq!(versions_after.len(), 1);
    assert_eq!(versions_after[0].version, 2);
    let latest = store.latest_artifact(id, "tree").await.unwrap().unwrap();
    assert_eq!(latest.value, serde_json::json!({"v": 2}));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn retention_delete_turns_after_without_keep_leaves_artifacts() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;
    for i in 1..=2 {
        store
            .append_turn_with_artifacts(
                id,
                TurnInput::new(TurnRole::Assistant, serde_json::json!({"i": i})),
                &[ArtifactWrite::new("tree", serde_json::json!({"v": i}))],
            )
            .await
            .unwrap();
    }

    let report = store
        .sweep_retention(
            "page-builder",
            &RetentionPolicy::DeleteTurnsAfter {
                ttl: chrono::Duration::days(1),
                keep_latest_artifact: false,
            },
            chrono::Utc::now() + chrono::Duration::days(365),
        )
        .await
        .unwrap();
    assert_eq!(report.turns_deleted, 2);
    assert_eq!(report.artifacts_deleted, 0);

    let versions = store.list_artifact_versions(id, "tree").await.unwrap();
    assert_eq!(versions.len(), 2);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn retention_zero_ttl_skips_recent_rows() {
    let (pool, _guard) = boot().await;
    let store = PostgresAgentSessionStore::new(pool);
    let id = fresh_session(&store).await;
    let report = store
        .sweep_retention(
            "page-builder",
            &RetentionPolicy::DeleteAfter {
                ttl: chrono::Duration::hours(1),
            },
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    assert_eq!(report.sessions_deleted, 0);
    assert!(store.get(id).await.unwrap().is_some());
}
