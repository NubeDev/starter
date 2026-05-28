//! Integration coverage for the Reversible-aware tool dispatch
//! wrapper. Pins the end-to-end loop:
//!
//!   1. A fake [`Tool`] implements [`ReversibleTool`] and returns a
//!      `ChangeDraft` describing its mutation.
//!   2. A fake [`Reversible`] is registered against the same kind.
//!   3. [`UndoDispatcher`] runs the tool, then `record_if_reversible`
//!      persists a `starter_changes` row through the live
//!      `SqliteChangeRecorder`.
//!   4. Replaying that row through `Reversible::apply_inverse`
//!      reaches our fake — proving the inverse path is wired and the
//!      row carries the snapshot the fake needs to roll back.
//!
//! Lives at the rubix-agent layer (not in `starter-undo`) because the
//! seam under test is the agent-side wiring: dispatcher + recorder +
//! registry. The starter-undo unit test covers the helper in
//! isolation.

use std::sync::{Arc, Mutex as StdMutex};

use async_trait::async_trait;
use serde_json::Value;
use starter_changelog::{ChangeFilter, ChangeLog};
use starter_changelog_sqlite::{
    migration_source as changelog_migration_source, SqliteChangeLog, SqliteChangeRecorder,
};
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Actor, Change, ChangeTx, Op, Reversible};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::Result;
use starter_store_sqlite::{migrate, testing::ephemeral};
use starter_undo::{ChangeDraft, ReversibleRegistry};

use rubix_tools::undo::dispatch::{ReversibleTool, StaticActor, UndoDispatcher};

/// Fake tool: every invocation updates widget `w-1` from `before` to
/// `after`, both echoed back in the response so `change_for` can
/// reconstruct the draft.
struct WidgetUpdateTool;

#[async_trait]
impl Tool for WidgetUpdateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "test.widgets.update".to_owned(),
            description: "fake".to_owned(),
            input_schema: serde_json::json!({"type": "object"}),
        }
    }
    async fn invoke(&self, input: Value) -> Result<Value> {
        // Echo input back as the "after" snapshot.
        Ok(serde_json::json!({
            "before": {"name": "old"},
            "after": input,
        }))
    }
}

impl ReversibleTool for WidgetUpdateTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let before = output.get("before")?.clone();
        let after = output.get("after")?.clone();
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: "widgets".into(),
                id: Some("w-1".into()),
                owner: None,
                tenant: None,
            },
            op: Op::Update,
            before: Some(before),
            after: Some(after),
            resource_version: None,
            correlation: None,
        })
    }
}

#[derive(Default)]
struct CapturingReversible {
    seen: Arc<StdMutex<Vec<Change>>>,
}

#[async_trait]
impl Reversible for CapturingReversible {
    fn kind(&self) -> &'static str {
        "widgets"
    }
    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        self.seen.lock().unwrap().push(ch.clone());
        Ok(())
    }
    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        self.seen.lock().unwrap().push(ch.clone());
        Ok(())
    }
    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: Value,
    ) -> Result<Vec<ResourceRef>> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn dispatch_records_change_and_inverse_round_trips() {
    // Wire the live SQLite recorder + log so this exercises the
    // same code path production uses (Postgres has the same trait
    // shape; the test layer is interchangeable on purpose).
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(changelog_migration_source())
        .run()
        .await
        .expect("apply changelog migration");
    let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
    let log = SqliteChangeLog::new(pool.clone());

    let reversible = Arc::new(CapturingReversible::default());
    let registry = Arc::new(ReversibleRegistry::new().insert(reversible.clone()));

    let actor = Actor::User {
        subject: "alice".into(),
    };
    let dispatcher = UndoDispatcher::new(
        Arc::new(WidgetUpdateTool),
        registry.clone(),
        recorder.clone(),
        Arc::new(StaticActor(actor.clone())),
    );

    let input = serde_json::json!({"name": "new"});
    let (output, group) = dispatcher
        .invoke_with_group(input)
        .await
        .expect("dispatch succeeds");
    assert_eq!(
        output.get("after"),
        Some(&serde_json::json!({"name": "new"}))
    );
    let group = group.expect("kind is registered so a group id is returned");

    // The recorder wrote one row, scoped to the group the dispatcher
    // returned.
    let page = log
        .list(&{
            let mut f = ChangeFilter::default();
            f.group_id = Some(group.clone());
            f
        })
        .await
        .expect("list rows");
    assert_eq!(page.items.len(), 1, "exactly one change row was recorded");
    let recorded = page.items[0].clone();
    assert_eq!(recorded.resource.kind, "widgets");
    assert_eq!(recorded.op, Op::Update);
    assert_eq!(recorded.before, Some(serde_json::json!({"name": "old"})));
    assert_eq!(recorded.after, Some(serde_json::json!({"name": "new"})));

    // Inverse path: hand the row to the registered Reversible. This
    // is the same call `UndoService::undo` would make.
    let r = registry.get("widgets").expect("registered");
    r.apply_inverse(&recorded).await.expect("inverse runs");
    let seen = reversible.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 1, "inverse reached the registered impl");
    assert_eq!(seen[0].before, Some(serde_json::json!({"name": "old"})));
}

/// Pins proposal §3.4: "any new mutation by an actor clears
/// that actor's redo stack". Without this, an undo → mutate →
/// redo sequence would re-apply the *original* mutation on top of
/// the new one, silently corrupting the row.
///
/// Construct the dispatcher with `with_cursor`, push a fake redo
/// entry onto the cursor for our actor, run one successful
/// mutation, and assert the cursor was cleared.
#[tokio::test]
async fn dispatch_clears_redo_stack_on_successful_mutation() {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(changelog_migration_source())
        .run()
        .await
        .expect("apply changelog migration");
    let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));

    let reversible = Arc::new(CapturingReversible::default());
    let registry = Arc::new(ReversibleRegistry::new().insert(reversible.clone()));

    let actor = Actor::User {
        subject: "alice".into(),
    };

    // Seed the cursor with a redo entry for alice — stands in for
    // "alice just undid a previous mutation". The new mutation
    // below must clear it.
    let cursor: Arc<dyn starter_undo::UndoCursor> =
        Arc::new(starter_undo::InMemoryUndoCursor::new());
    cursor
        .push_redo(
            &actor,
            starter_spi::changelog::GroupId("g-prior".to_owned()),
        )
        .await
        .expect("seed redo");
    assert!(
        cursor.peek_redo(&actor).await.unwrap().is_some(),
        "precondition: cursor has a redo entry"
    );

    let dispatcher = UndoDispatcher::with_cursor(
        Arc::new(WidgetUpdateTool),
        registry.clone(),
        recorder.clone(),
        Arc::new(StaticActor(actor.clone())),
        cursor.clone(),
    );

    let (_output, group) = dispatcher
        .invoke_with_group(serde_json::json!({"name": "new"}))
        .await
        .expect("dispatch succeeds");
    assert!(group.is_some(), "mutation landed a changelog row");

    assert!(
        cursor.peek_redo(&actor).await.unwrap().is_none(),
        "new mutation must clear actor's redo stack (proposal §3.4)"
    );
}

/// Sibling guard: a verb whose kind is *not* registered as
/// reversible must not clear the redo stack — read-only and
/// non-undoable verbs are background noise to the cursor.
#[tokio::test]
async fn dispatch_preserves_redo_stack_on_unregistered_kind() {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(changelog_migration_source())
        .run()
        .await
        .expect("apply changelog migration");
    let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));

    // Empty registry: WidgetUpdateTool's "widgets" kind is not
    // registered, so `record_if_reversible` short-circuits to
    // `Ok(None)` and the dispatcher must NOT touch the cursor.
    let registry = Arc::new(ReversibleRegistry::new());

    let actor = Actor::User {
        subject: "alice".into(),
    };
    let cursor: Arc<dyn starter_undo::UndoCursor> =
        Arc::new(starter_undo::InMemoryUndoCursor::new());
    let preserved_group = starter_spi::changelog::GroupId("g-keep".to_owned());
    cursor
        .push_redo(&actor, preserved_group.clone())
        .await
        .expect("seed redo");

    let dispatcher = UndoDispatcher::with_cursor(
        Arc::new(WidgetUpdateTool),
        registry,
        recorder,
        Arc::new(StaticActor(actor.clone())),
        cursor.clone(),
    );

    let (_output, group) = dispatcher
        .invoke_with_group(serde_json::json!({"name": "new"}))
        .await
        .expect("dispatch succeeds");
    assert!(group.is_none(), "unregistered kind → no row recorded");

    let top = cursor.peek_redo(&actor).await.unwrap();
    assert_eq!(
        top.as_ref(),
        Some(&preserved_group),
        "redo stack must be preserved when the verb didn't land a reversible mutation"
    );
}
