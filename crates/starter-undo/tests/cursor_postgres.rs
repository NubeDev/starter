//! Persistence + concurrency tests for [`PgUndoCursor`].
//!
//! Exercises:
//! - basic LIFO push/peek/pop round-trip,
//! - `clear_redo` isolation between actors,
//! - "stack survives reopen" smoke (new cursor handle against the
//!   same pool), and
//! - epoch CAS under simulated concurrent writers (two cursors
//!   racing push on the same actor must both succeed and end up
//!   with both entries on the stack).
//!
//! **`#[ignore]`** by default — requires Docker. Run with
//! `cargo test -p starter-undo --features postgres -- --ignored`.

use std::sync::Arc;

use starter_spi::changelog::{Actor, GroupId};
use starter_store_postgres::{migrate, testing::with_database};
use starter_undo::cursor_postgres::{migration_source, PgUndoCursor};
use starter_undo::UndoCursor;

async fn setup() -> (PgUndoCursor, starter_store_postgres::testing::ContainerGuard) {
    let (pool, guard) = with_database().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");
    (PgUndoCursor::new(pool), guard)
}

#[tokio::test]
#[ignore = "requires docker"]
async fn push_peek_pop_round_trip() {
    let (cursor, _guard) = setup().await;
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

    assert_eq!(cursor.pop_redo(&alice).await.unwrap().unwrap().0, "g2");
    assert_eq!(cursor.pop_redo(&alice).await.unwrap().unwrap().0, "g1");
    assert!(cursor.pop_redo(&alice).await.unwrap().is_none(), "empty");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn clear_redo_wipes_only_that_actor() {
    let (cursor, _guard) = setup().await;
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
#[ignore = "requires docker"]
async fn stack_survives_new_cursor_handle() {
    // Simulates a process restart: the second cursor sees what the
    // first wrote because the state is in Postgres, not in process
    // memory.
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let actor = Actor::User {
        subject: "alice".into(),
    };

    {
        let cursor = PgUndoCursor::new(pool.clone());
        cursor
            .push_redo(&actor, GroupId("survives".into()))
            .await
            .expect("push");
    }

    let fresh = PgUndoCursor::new(pool);
    assert_eq!(
        fresh.peek_redo(&actor).await.unwrap().unwrap().0,
        "survives",
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn concurrent_push_both_land() {
    // Two cursors against the same pool push concurrently. The CAS
    // retry loop must reconcile so both groups end up on the stack —
    // proving the epoch protects the read-modify-write window even
    // when wall-clock writes interleave.
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(migration_source())
        .run()
        .await
        .expect("migration");

    let alice = Actor::User {
        subject: "alice".into(),
    };
    let c1 = Arc::new(PgUndoCursor::new(pool.clone()));
    let c2 = Arc::new(PgUndoCursor::new(pool.clone()));

    let alice1 = alice.clone();
    let alice2 = alice.clone();
    let h1 = tokio::spawn({
        let c1 = c1.clone();
        async move { c1.push_redo(&alice1, GroupId("from-c1".into())).await }
    });
    let h2 = tokio::spawn({
        let c2 = c2.clone();
        async move { c2.push_redo(&alice2, GroupId("from-c2".into())).await }
    });
    h1.await.unwrap().expect("c1 push");
    h2.await.unwrap().expect("c2 push");

    // Pop both — order is whichever CAS landed second on top.
    let top = c1.pop_redo(&alice).await.unwrap().expect("top");
    let next = c1.pop_redo(&alice).await.unwrap().expect("next");
    let observed = [top.0, next.0];
    assert!(
        observed.contains(&"from-c1".into()) && observed.contains(&"from-c2".into()),
        "both pushes must land; got {observed:?}",
    );
    assert!(c1.pop_redo(&alice).await.unwrap().is_none(), "empty");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn agent_actor_keyed_by_run_id_not_model() {
    let (cursor, _guard) = setup().await;
    let cursor: Arc<dyn UndoCursor> = Arc::new(cursor);

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
        model: "claude-3-sonnet".into(),
    };
    assert_eq!(
        cursor.peek_redo(&after).await.unwrap().unwrap().0,
        "g",
        "run_id alone keys the stack",
    );
}
