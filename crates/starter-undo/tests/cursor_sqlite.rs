//! Persistence test for [`SqliteUndoCursor`].
//!
//! Exercises the four cursor ops + a "survives reopen" smoke (we
//! drop the cursor handle and build a fresh one against the same
//! pool, then read the stack back) so a reader can see what
//! cross-process undo guarantees look like.

use std::sync::Arc;

use starter_spi::changelog::{Actor, GroupId};
use starter_store_sqlite::{migrate, testing::ephemeral};
use starter_undo::cursor_sqlite::{migration_source, SqliteUndoCursor};
use starter_undo::UndoCursor;

#[tokio::test]
async fn push_peek_pop_round_trip() {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let cursor = SqliteUndoCursor::new(pool.clone());
    let alice = Actor::User {
        subject: "alice".into(),
    };

    assert!(cursor.peek_redo(&alice).await.unwrap().is_none());

    cursor
        .push_redo(&alice, GroupId("g1".into()))
        .await
        .expect("push 1");
    cursor
        .push_redo(&alice, GroupId("g2".into()))
        .await
        .expect("push 2");

    let peek = cursor.peek_redo(&alice).await.unwrap();
    assert_eq!(peek.unwrap().0, "g2", "LIFO: newest on top");

    let popped = cursor.pop_redo(&alice).await.unwrap();
    assert_eq!(popped.unwrap().0, "g2");
    let popped = cursor.pop_redo(&alice).await.unwrap();
    assert_eq!(popped.unwrap().0, "g1");
    assert!(cursor.pop_redo(&alice).await.unwrap().is_none(), "empty");
}

#[tokio::test]
async fn clear_redo_wipes_only_that_actor() {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let cursor = SqliteUndoCursor::new(pool.clone());
    let alice = Actor::User {
        subject: "alice".into(),
    };
    let bob = Actor::User {
        subject: "bob".into(),
    };

    cursor
        .push_redo(&alice, GroupId("a1".into()))
        .await
        .unwrap();
    cursor.push_redo(&bob, GroupId("b1".into())).await.unwrap();
    cursor.clear_redo(&alice).await.unwrap();

    assert!(cursor.peek_redo(&alice).await.unwrap().is_none());
    assert_eq!(cursor.peek_redo(&bob).await.unwrap().unwrap().0, "b1");
}

#[tokio::test]
async fn stack_survives_new_cursor_handle() {
    // Simulates a process restart: drop the original `SqliteUndoCursor`
    // handle, build a fresh one against the same pool, and read the
    // stack back — proving the state is in the DB, not in process
    // memory.
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let actor = Actor::User {
        subject: "alice".into(),
    };

    {
        let cursor = SqliteUndoCursor::new(pool.clone());
        cursor
            .push_redo(&actor, GroupId("survives".into()))
            .await
            .expect("push");
    }
    // Original handle dropped.

    let fresh = SqliteUndoCursor::new(pool.clone());
    let peek = fresh.peek_redo(&actor).await.unwrap();
    assert_eq!(peek.unwrap().0, "survives");
}

#[tokio::test]
async fn agent_actor_keyed_by_run_id_not_model() {
    // The actor-key helper drops the agent model so a re-attached
    // agent run resumes its own stack even if the model rev changes.
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let cursor: Arc<dyn UndoCursor> = Arc::new(SqliteUndoCursor::new(pool.clone()));

    let before = Actor::Agent {
        run_id: "run-42".into(),
        model: "claude-3-opus".into(),
    };
    cursor
        .push_redo(&before, GroupId("g".into()))
        .await
        .unwrap();

    let after = Actor::Agent {
        run_id: "run-42".into(),
        model: "claude-3-sonnet".into(), // model swapped
    };
    let peek = cursor.peek_redo(&after).await.unwrap();
    assert_eq!(peek.unwrap().0, "g", "run_id alone keys the stack");
}
