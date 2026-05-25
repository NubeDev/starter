//! Phase C.3 — live tick + hot-edit + restart-persistence integration
//! coverage for the always-on flow runtime.
//!
//! Boots [`rubix_agent::boot::build_flow_runtime`] against a
//! testcontainers Postgres pool (selects the durable
//! `PgNodeStateStore` backend; the legacy SQLite path was removed
//! per `rubix/docs/scope/sqlite-to-postgres.md`). Three scenarios
//! land here, each one a present-tense assertion against the
//! pieces that already exist on `master`:
//!
//! 1. **Live tick** — deploy a `starter.flow.counter`-shaped flow
//!    with cron `*/1 * * * * *` (every second), drive three
//!    "ticks" by invoking the counter through the production
//!    `NodeBehavior::invoke` chokepoint with the same
//!    [`NodeStateStore`] backing the agent boot, fan out a
//!    matching `FlowEvent::NodeEmitted` into the shared
//!    [`FlowSubscriptionRegistry`] per tick, sleep 3s, then
//!    assert `node_state.count >= 3` *and* the SSE-side receiver
//!    drained at least three [`FlowEvent::NodeEmitted`] frames.
//!
//! 2. **Hot edit** — swap the counter body for a fresh one with
//!    `step = 10` (this is the production hot-reload path:
//!    `DefinitionManager` rebuilds the kind body when settings
//!    classify as `EditKind::Settings`), invoke once more, assert
//!    the next tick's count jumps by exactly 10.
//!
//! 3. **Restart persistence** — drop the live `FlowRuntime`
//!    (including its `Arc<dyn NodeStateStore>`), rebuild via
//!    `boot::build_flow_runtime` pointed at the *same* SQLite
//!    file, invoke the counter, and assert the new runner reads
//!    the prior count and steps from there.
//!
//! What this test deliberately does **not** drive: the full
//! `FlowRunner::start` propagator. The always-on engine-pump that
//! wires every live revision's `RunHandle::events_tx` into the
//! per-flow `FlowSubscriptionRegistry` is the follow-up stage
//! (see `boot/flow_runtime.rs` module docs). Until that lands,
//! this test exercises the same chokepoints the pump will reuse
//! (the counter body, the state store, the subscription
//! registry) so the integration contract is pinned end-to-end
//! before the wiring change.
//!
//! Marked `#[ignore]` because the PG backend selection requires
//! Docker. Run with:
//!
//! ```bash
//! cargo test -p rubix-agent --test flow_live_tick_test -- --ignored
//! ```

use std::sync::Arc;
use std::time::Duration;

use rubix_agent::boot::config::FlowRuntimeConfig;
use rubix_agent::boot::flow_runtime::{build, FlowSubscriptionRegistry};

use starter_flow_nodes::counter::{Counter, CounterSettings, OUT_SLOT, STATE_KEY};
use starter_flow_spi::flow::{FlowEvent, FlowId, RunId};
use starter_flow_spi::node::{NodeBehavior, NodeCtx, NodeId, SlotMap, SlotValue};
use starter_flow_spi::skill::SkillSelection;
use starter_flow_spi::state::{NodeStateKey, NodeStateStore};
use starter_flow_spi::Cancel;

use starter_store_postgres::testing::with_database;

const FLOW_ID: &str = "com.rubix.live-tick-demo";
const NODE_ID: &str = "com.rubix.live-tick-demo.counter";
/// Cron from the stage spec: every second, 6-field grammar that
/// the durable scheduler's `starter-cron` parser expects.
#[allow(dead_code)]
const CRON_EVERY_SECOND: &str = "*/1 * * * * *";

/// Test-only [`Cancel`] implementation that never trips. Mirrors
/// the `NoCancel` helper every `starter-flow-nodes` unit test
/// uses (e.g. `counter_invoke_test.rs`).
struct NoCancel;

impl Cancel for NoCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

fn flow_id() -> FlowId {
    FlowId::new(FLOW_ID).expect("FLOW_ID is a valid reverse-DNS id")
}

fn node_id() -> NodeId {
    NodeId::new(NODE_ID).expect("NODE_ID is a valid reverse-DNS id")
}

fn state_key() -> NodeStateKey {
    NodeStateKey::new(flow_id(), node_id(), STATE_KEY)
        .expect("counter state key constructs")
}

/// Read the persisted `count` (`None` if unset).
async fn read_count(store: &dyn NodeStateStore) -> Option<i64> {
    let v = store.get(&state_key()).await.expect("state get");
    v.map(|v| {
        std::str::from_utf8(&v.bytes)
            .expect("count bytes are utf-8")
            .parse::<i64>()
            .expect("count bytes parse as i64")
    })
}

/// Drive one tick: invoke `counter` through the production
/// `NodeBehavior::invoke` chokepoint, then forward the
/// `NodeEmitted` event into the shared
/// [`FlowSubscriptionRegistry`] exactly as the (follow-up)
/// engine-pump will.
async fn tick_once(
    counter: &Counter,
    store: &dyn NodeStateStore,
    subs: &FlowSubscriptionRegistry,
) -> i64 {
    let flow = flow_id();
    let node = node_id();
    let run = RunId::new();
    let cancel = NoCancel;
    let ctx = NodeCtx::with_flow(
        &flow,
        run,
        &node,
        &cancel,
        SkillSelection::NONE,
        store,
    );
    let out: SlotMap = counter
        .invoke(ctx, SlotMap::new())
        .await
        .expect("counter invoke succeeds");
    let value = out
        .get(OUT_SLOT)
        .cloned()
        .expect("counter emits on its `out` slot");
    let int = match value.clone() {
        SlotValue::Int(n) => n,
        other => panic!("counter `out` is not Int: {other:?}"),
    };
    // Forward through the per-flow broadcast — this stands in
    // for the engine-side run pump until it lands.
    let tx = subs.sender(&flow).await;
    let _ = tx.send(FlowEvent::NodeEmitted {
        run,
        node: node.clone(),
        slot: OUT_SLOT.to_owned(),
        value,
    });
    int
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires docker"]
async fn live_tick_hot_edit_and_restart_persistence() {
    // ----- 1. Boot the backing store ---------------------------------
    // testcontainers Postgres pool — `flow_runtime::build` now
    // takes the already-open pool from the caller and runs the
    // upstream `FLOW_MIGRATION_SOURCE` against it to provision
    // the `node_state` table. The container started here owns
    // the database lifecycle for the test.
    let (pg_pool, _pg_guard) = with_database().await;

    let cfg = FlowRuntimeConfig::default();

    // ----- 2. First runtime: live-tick + SSE drain --------------------
    let rt = build(Some(pg_pool.clone()), &cfg)
        .await
        .expect("FlowRuntime builds against Postgres");
    let store: Arc<dyn NodeStateStore> = rt.state_store.clone();
    let subs = rt.subscriptions.clone();

    // SSE-side subscriber, subscribed *before* any tick so the
    // first emit is guaranteed observable (matches the
    // subscribe-before-send contract the SSE route relies on).
    let mut sse_rx = subs.subscribe_or_create(&flow_id()).await;

    // Default settings — step=1, initial=0, reset_on_redeploy=false.
    let counter_v1 = Counter::with_settings(CounterSettings::default());

    // Drive three ticks across ~3s. Cron `*/1 * * * * *` would
    // fire roughly once per second in the always-on mounter; we
    // drive the same chokepoint directly so the assertion does
    // not race the scheduler tick interval.
    for _ in 0..3 {
        tick_once(&counter_v1, store.as_ref(), subs.as_ref()).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // Sleep 3s per the stage spec so the assertion would still
    // hold against a real cron-driven mounter.
    tokio::time::sleep(Duration::from_secs(3)).await;

    let count_after_three = read_count(store.as_ref()).await.expect("count present");
    assert!(
        count_after_three >= 3,
        "live-tick: count must reach >= 3 after three ticks, saw {count_after_three}",
    );

    // Drain the SSE side and assert >= 3 NodeEmitted frames.
    let mut emitted = 0usize;
    loop {
        match sse_rx.try_recv() {
            Ok(FlowEvent::NodeEmitted { .. }) => emitted += 1,
            Ok(_) => {}
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
        }
    }
    assert!(
        emitted >= 3,
        "live-tick: SSE receiver must see >= 3 NodeEmitted frames, saw {emitted}",
    );

    // ----- 3. Hot edit: redeploy with step = 10 -----------------------
    let counter_v2 = Counter::with_settings(CounterSettings {
        step: 10,
        ..CounterSettings::default()
    });
    let before_edit = count_after_three;
    let after_edit = tick_once(&counter_v2, store.as_ref(), subs.as_ref()).await;
    assert_eq!(
        after_edit,
        before_edit + 10,
        "hot-edit: next tick must jump by step=10 (before={before_edit}, after={after_edit})",
    );

    // Drop the first runtime — including its Arc<dyn
    // NodeStateStore> handle — to mirror a process restart.
    let count_pre_restart = read_count(store.as_ref()).await.expect("count present");
    drop(sse_rx);
    drop(subs);
    drop(store);
    drop(rt);

    // ----- 4. Restart: rebuild against the same PG database -----------
    let rt2 = build(Some(pg_pool.clone()), &cfg)
        .await
        .expect("FlowRuntime rebuilds against the same Postgres database");
    let store2: Arc<dyn NodeStateStore> = rt2.state_store.clone();
    let subs2 = rt2.subscriptions.clone();

    // Without touching the file directly, the fresh runner must
    // observe the prior count.
    let resumed_count = read_count(store2.as_ref())
        .await
        .expect("count survives the restart");
    assert_eq!(
        resumed_count, count_pre_restart,
        "restart: rebuilt runtime sees prior count from sqlite",
    );

    // And the next tick increments from there (step=1 again — a
    // fresh kind body is constructed on restart, the persisted
    // value is what carries across).
    let counter_v3 = Counter::with_settings(CounterSettings::default());
    let after_restart = tick_once(&counter_v3, store2.as_ref(), subs2.as_ref()).await;
    assert_eq!(
        after_restart,
        resumed_count + 1,
        "restart: next tick after rebuild advances by step=1",
    );
}
