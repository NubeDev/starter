//! End-to-end coverage for the SQLite changelog backend + undo.
//!
//! Records two changes inside one recorder transaction, lists them,
//! then walks them through `UndoService::{undo, redo}` against a
//! fake `Reversible` impl whose call log we assert.

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use starter_changelog::{ChangeFilter, ChangeLog};
use starter_changelog_sqlite::{migration_source, SqliteChangeLog, SqliteChangeRecorder};
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{
    Actor, Change, ChangeId, ChangeRecorder, ChangeTx, GroupId, Op, Reversible,
};
use starter_spi::{Error, Result};
use starter_store_sqlite::{migrate, testing::ephemeral};
use starter_undo::{ReversibleRegistry, UndoService};

/// Fake note reversible. Records every call so the test can assert
/// ordering.
#[derive(Default, Clone)]
struct FakeNote {
    calls: Arc<Mutex<Vec<(String, ChangeId)>>>,
}

impl FakeNote {
    fn calls(&self) -> Vec<(String, ChangeId)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl Reversible for FakeNote {
    fn kind(&self) -> &'static str {
        "note"
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push(("inverse".into(), ch.id.clone()));
        Ok(())
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push(("forward".into(), ch.id.clone()));
        Ok(())
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "clone_with not exercised by this test".into(),
        })
    }
}

fn note_change(
    op: Op,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
) -> Change {
    Change {
        id: ChangeId(String::new()), // recorder overwrites
        at: Utc::now(),
        actor: Actor::User {
            subject: "alice".into(),
        },
        resource: ResourceRef::row("note", "n1"),
        resource_version: Some(1),
        op,
        before,
        after,
        patch: None,
        group_id: GroupId(String::new()), // recorder overwrites
        correlation: None,
    }
}

#[tokio::test]
async fn record_list_undo_redo() {
    // 1. fresh in-memory db + namespaced migration.
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let recorder = SqliteChangeRecorder::new(pool.clone());
    let log: Arc<dyn ChangeLog> = Arc::new(SqliteChangeLog::new(pool.clone()));

    // 2. record two rows under one transaction. We capture the ids
    // through a shared mutex; the boxed-closure shape doesn't allow
    // returning values directly.
    let recorded: Arc<Mutex<Vec<ChangeId>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded_inner = recorded.clone();

    recorder
        .transaction(Box::new(move |tx| {
            let recorded = recorded_inner.clone();
            Box::pin(async move {
                let id1 = tx
                    .record(note_change(
                        Op::Create,
                        None,
                        Some(serde_json::json!({"text": "hello"})),
                    ))
                    .await?;
                // Tiny delay so `at` strictly increases between rows.
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                let id2 = tx
                    .record(note_change(
                        Op::Update,
                        Some(serde_json::json!({"text": "hello"})),
                        Some(serde_json::json!({"text": "hi"})),
                    ))
                    .await?;
                recorded.lock().unwrap().extend([id1, id2]);
                Ok(())
            })
        }))
        .await
        .expect("recorder transaction");

    let recorded = recorded.lock().unwrap().clone();
    assert_eq!(recorded.len(), 2);

    // 3. list under the alice filter; assert both rows present and
    // sharing one group_id.
    let page = log
        .list(&ChangeFilter {
            actor_kind: Some("user".into()),
            actor_id: Some("alice".into()),
            ..Default::default()
        })
        .await
        .expect("list");
    assert_eq!(page.items.len(), 2);
    let groups: Vec<_> = page.items.iter().map(|c| c.group_id.clone()).collect();
    assert_eq!(groups[0].0, groups[1].0, "rows in one tx share group_id");

    // 4. undo through registry + service.
    let note = Arc::new(FakeNote::default());
    let registry = Arc::new(ReversibleRegistry::new().insert(note.clone() as Arc<dyn Reversible>));
    let undo = UndoService::new(log.clone(), registry);

    let actor = Actor::User {
        subject: "alice".into(),
    };

    let undone_group = undo.undo(&actor).await.expect("undo");
    assert_eq!(undone_group.0, groups[0].0);

    let calls = note.calls();
    assert_eq!(calls.len(), 2, "two rows undone");
    assert!(
        calls.iter().all(|(k, _)| k == "inverse"),
        "all undo calls are inverse"
    );
    // Inverse order: last recorded row first.
    assert_eq!(calls[0].1 .0, recorded[1].0, "newest row undone first");
    assert_eq!(calls[1].1 .0, recorded[0].0, "oldest row undone last");

    // Redo — should call apply_forward in `at` ascending order.
    let redone_group = undo.redo(&actor).await.expect("redo");
    assert_eq!(redone_group.0, undone_group.0);
    let calls = note.calls();
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[2].0, "forward");
    assert_eq!(calls[3].0, "forward");
    assert_eq!(calls[2].1 .0, recorded[0].0, "oldest row redone first");
    assert_eq!(calls[3].1 .0, recorded[1].0, "newest row redone last");

    // Undo again should target the same group (since it's the only
    // one) — the redo stack was popped by `redo`.
    let undone_again = undo.undo(&actor).await.expect("second undo");
    assert_eq!(undone_again.0, undone_group.0);

    // A *third* undo finds nothing past the top-of-redo-stack and
    // returns NotFound.
    let err = undo
        .undo(&actor)
        .await
        .expect_err("third undo has no target");
    assert!(matches!(err, Error::NotFound { .. }), "got {err:?}");
}

/// `ChangeRecorder::forget` tombstones payloads but preserves the
/// row identity required for replay integrity (SCOPE §"Security &
/// privacy").
#[tokio::test]
async fn forget_tombstones_payloads_but_keeps_skeleton() {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let recorder = SqliteChangeRecorder::new(pool.clone());
    let log: Arc<dyn ChangeLog> = Arc::new(SqliteChangeLog::new(pool.clone()));

    recorder
        .transaction(Box::new(move |tx| {
            Box::pin(async move {
                tx.record(note_change(
                    Op::Create,
                    None,
                    Some(serde_json::json!({"text": "PII"})),
                ))
                .await?;
                tx.record(note_change(
                    Op::Update,
                    Some(serde_json::json!({"text": "PII"})),
                    Some(serde_json::json!({"text": "more PII"})),
                ))
                .await?;
                Ok(())
            })
        }))
        .await
        .expect("seed");

    // Record a row on a *different* resource id so we can prove the
    // forget call is scoped.
    recorder
        .transaction(Box::new(move |tx| {
            Box::pin(async move {
                let mut other = note_change(
                    Op::Create,
                    None,
                    Some(serde_json::json!({"text": "untouched"})),
                );
                other.resource = ResourceRef::row("note", "n2");
                tx.record(other).await?;
                Ok(())
            })
        }))
        .await
        .expect("seed other");

    let rows = recorder
        .forget(&ResourceRef::row("note", "n1"))
        .await
        .expect("forget");
    assert_eq!(rows, 2, "two n1 rows tombstoned");

    let page = log
        .list(&ChangeFilter {
            resource_kind: Some("note".into()),
            resource_id: Some("n1".into()),
            ..Default::default()
        })
        .await
        .expect("list n1");
    assert_eq!(page.items.len(), 2, "rows preserved");
    for ch in &page.items {
        assert!(ch.before.is_none(), "before nulled");
        assert!(ch.after.is_none(), "after nulled");
        assert!(ch.patch.is_none(), "patch nulled");
        // Skeleton survives.
        assert!(!ch.id.0.is_empty());
        assert!(!ch.group_id.0.is_empty());
        assert!(matches!(ch.actor, Actor::User { .. }));
    }

    // The other resource is untouched.
    let page = log
        .list(&ChangeFilter {
            resource_kind: Some("note".into()),
            resource_id: Some("n2".into()),
            ..Default::default()
        })
        .await
        .expect("list n2");
    assert_eq!(page.items.len(), 1);
    assert!(page.items[0].after.is_some(), "n2 payload preserved");

    // Tombstoning is idempotent.
    let rows_again = recorder
        .forget(&ResourceRef::row("note", "n1"))
        .await
        .expect("forget again");
    assert_eq!(rows_again, 2, "still matches by id");
}
