//! Goal 2 (user-admin) integration coverage.
//!
//! Drives the user-admin write verbs through the same
//! [`UndoDispatcher`] seam the agent loop uses, then exercises
//! `rubix.undo.last` to walk the most recent change back through
//! the registered [`UserReversible`]. Two scenarios:
//!
//!   1. `create_via_dispatcher_persists_row_and_emits_diagnostic` —
//!      fires `rubix.user.create`, asserts the row landed in the
//!      backing store and the response `Diagnostic` carries
//!      `code = rubix.user.created`. A `Change` row is recorded in
//!      the changelog by the dispatcher.
//!   2. `undo_last_reverses_user_disable_back_to_enabled` —
//!      creates + disables the user, asserts `disabled_at_ms`
//!      flips, then fires `rubix.undo.last` and asserts the
//!      `UserReversible` walked the disable back so the user is
//!      enabled again.
//!
//! Backing store note: the PG-backed `UserAdminStore` impl lands
//! in a follow-up phase (see
//! [docs/design/user-admin/](../../../docs/design/user-admin/README.md)).
//! Until then the `InMemoryUserStore` stands in — the trait shape
//! is the contract, so the production swap is a one-line change in
//! the agent boot wiring and the assertions below stay green.
//! Equivalent end-to-end coverage through the `rubix-admin mcp`
//! transport will follow once the user verbs are wired into
//! `boot::mcp::register::build_flow_registry`.

use std::sync::Arc;

use serde_json::json;
use starter_changelog::{filter_for_actor, ChangeLog};
use starter_changelog_sqlite::{
    migration_source as changelog_migration_source, SqliteChangeLog, SqliteChangeRecorder,
};
use starter_spi::changelog::{Actor, Op};
use starter_spi::tool::Tool;
use starter_store_sqlite::{migrate, testing::ephemeral};
use starter_undo::{ReversibleRegistry, UndoService};

use rubix_spi::dto::user::create::UserCreateResponse;
use rubix_spi::dto::user::disable::UserDisableResponse;
use rubix_tools::undo::dispatch::{StaticActor, UndoDispatcher};
use rubix_tools::undo::last::UndoLastTool;
use rubix_tools::user::create::UserCreateTool;
use rubix_tools::user::disable::UserDisableTool;
use rubix_tools::user::store::{InMemoryUserStore, UserAdminStore, UserReversible, USER_KIND};

/// Shared wiring used by both scenarios. Returns the in-memory user
/// store, the undo-aware dispatchers for `create` / `disable`, the
/// `undo.last` verb, and a handle to the changelog so tests can
/// assert recorded rows.
async fn setup() -> Setup {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(changelog_migration_source())
        .run()
        .await
        .expect("apply changelog migration");

    let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
    let log: Arc<dyn ChangeLog> = Arc::new(SqliteChangeLog::new(pool.clone()));

    let store: Arc<dyn UserAdminStore> = Arc::new(InMemoryUserStore::new());
    let reversible = Arc::new(UserReversible::new(store.clone()));
    let registry = Arc::new(ReversibleRegistry::new().insert(reversible));

    let actor = Actor::User {
        subject: "ada@x".into(),
    };
    let actor_source = Arc::new(StaticActor(actor.clone()));

    let create = UndoDispatcher::new(
        Arc::new(UserCreateTool::new(store.clone())),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
    );
    let disable = UndoDispatcher::new(
        Arc::new(UserDisableTool::new(store.clone())),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
    );

    let undo_service = Arc::new(UndoService::new(log.clone(), registry.clone()));
    let undo_last = UndoLastTool::new(undo_service, actor_source);

    Setup {
        store,
        create,
        disable,
        undo_last,
        log,
        actor,
    }
}

struct Setup {
    store: Arc<dyn UserAdminStore>,
    create: UndoDispatcher<UserCreateTool>,
    disable: UndoDispatcher<UserDisableTool>,
    undo_last: UndoLastTool,
    log: Arc<dyn ChangeLog>,
    actor: Actor,
}

#[tokio::test]
async fn create_via_dispatcher_persists_row_and_emits_diagnostic() {
    let s = setup().await;

    let out = s
        .create
        .invoke(json!({"email": "ada@x", "role": "admin"}))
        .await
        .expect("create dispatch succeeds");
    let resp: UserCreateResponse =
        serde_json::from_value(out).expect("UserCreateResponse decodes");

    // Diagnostic code — verb output is structured, not a string.
    assert_eq!(
        resp.summary.code.as_str(),
        "rubix.user.created",
        "create emits rubix.user.created",
    );

    // Backing-store row (stands in for the PG row per the module
    // header — see follow-up note in docs/design/user-admin/).
    let row = s
        .store
        .find_by_email("ada@x")
        .await
        .expect("store lookup ok")
        .expect("user row persisted by create dispatch");
    assert_eq!(row.email, "ada@x");
    assert_eq!(row.role, "admin");
    assert!(row.disabled_at_ms.is_none());

    // Dispatcher recorded the change so undo can find it.
    let page = s
        .log
        .list(&filter_for_actor(&s.actor))
        .await
        .expect("list changelog rows");
    assert_eq!(page.items.len(), 1, "create dispatch recorded one change");
    let ch = &page.items[0];
    assert_eq!(ch.resource.kind, USER_KIND);
    assert_eq!(ch.op, Op::Create);
    assert_eq!(ch.resource.id.as_deref(), Some(resp.user_id.as_str()));
}

#[tokio::test]
async fn undo_last_reverses_user_disable_back_to_enabled() {
    let s = setup().await;

    // Seed: create then disable.
    let create_out = s
        .create
        .invoke(json!({"email": "ada@x", "role": "admin"}))
        .await
        .expect("create dispatch succeeds");
    let created: UserCreateResponse = serde_json::from_value(create_out).unwrap();

    let disable_out = s
        .disable
        .invoke(json!({"user_id": created.user_id.clone()}))
        .await
        .expect("disable dispatch succeeds");
    let disabled: UserDisableResponse = serde_json::from_value(disable_out).unwrap();
    assert_eq!(disabled.summary.code.as_str(), "rubix.user.disabled");
    assert!(!disabled.was_already_disabled);

    // Confirm the state actually flipped before undoing.
    let mid = s
        .store
        .get(&created.user_id)
        .await
        .unwrap()
        .expect("user present");
    assert!(
        mid.disabled_at_ms.is_some(),
        "disable verb set disabled_at_ms",
    );

    // Fire rubix.undo.last as the same actor — walks the most recent
    // group (the disable) back through UserReversible::apply_inverse.
    let undo_out = s
        .undo_last
        .invoke(json!({}))
        .await
        .expect("undo.last dispatch succeeds");
    assert!(
        undo_out.get("group_id").and_then(|v| v.as_str()).is_some(),
        "undo.last returns the undone group id; got {undo_out}",
    );

    // The UserReversible's Op::Update branch restores the prior
    // UserRow snapshot (disabled_at_ms = None) so the user is
    // enabled again. This is the contract documented in
    // docs/design/user-admin/ §"Snapshot shape".
    let after = s
        .store
        .get(&created.user_id)
        .await
        .unwrap()
        .expect("user still present after undo");
    assert!(
        after.disabled_at_ms.is_none(),
        "undo of disable re-enables the user; got {:?}",
        after.disabled_at_ms,
    );
    assert_eq!(after.email, "ada@x");
    assert_eq!(after.role, "admin");
}
