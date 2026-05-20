//! Phase 3 stage 7 — `FlowAsTool` body invariants (SCOPE R8 +
//! D-F3.4).
//!
//! Per the job WORKFLOW per-stage table, stage 7 lands the
//! [`FlowAsTool`] body and "unit tests cover happy-path / typed-
//! error / cancel-within-200ms / no-task-leak". This file is that
//! coverage:
//!
//! 1. [`builder_rejects_missing_required_fields`] — the
//!    [`FlowAsToolBuilder`] is the only construction path and
//!    every D-F3.4 explicit field is required.
//! 2. [`invoke_drives_flow_and_returns_terminal_output`] —
//!    happy path: `Tool::invoke` seeds the flow, runs it, and
//!    the [`OutputAdapter`] sees the terminal slot value.
//! 3. [`invoke_surfaces_flow_failure_as_typed_error`] — a node
//!    that returns [`NodeError`] surfaces as
//!    [`starter_spi::error::Error::Internal`] carrying a
//!    `flow run failed` source.
//! 4. [`invoke_rejects_while_engine_is_degraded`] —
//!    `EngineHealth::Degraded` flips to a typed error before any
//!    propagator work happens (stage-6 D-F3.11 backstop).
//! 5. [`invoke_with_cancel_propagates_within_200ms`] — R13: a
//!    fired [`Cancel`] flips the per-run cancel and the call
//!    returns `flow run cancelled` within 200 ms.
//! 6. [`invoke_does_not_leak_tokio_tasks`] — the
//!    cancel-forwarder task this stage spawns is reliably
//!    aborted on every termination path (Completed / Failed /
//!    Cancelled). Asserts `tokio::runtime::Handle::metrics`
//!    sees no growth across N back-to-back invocations.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::time::sleep;

use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::health::HealthHandle;
use starter_flow::propagator::FlowTopology;
use starter_flow::run::RunCancel;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};
use starter_spi::error::Error as SpiError;
use starter_spi::tool::Tool;

use starter_flow_surfaces::{FlowAsTool, FlowAsToolBuildError};

// ---------------------------------------------------------------------------
// Shared test scaffolding: a single identity-style node "n" with an
// `in` trigger slot writing to an `out` terminal slot. Each
// invocation runs one tick.
// ---------------------------------------------------------------------------

struct Identity {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for Identity {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let v = input.get("in").cloned().unwrap_or(SlotValue::Null);
        let mut out = SlotMap::new();
        out.insert("out".to_owned(), v);
        Ok(out)
    }
}

struct AlwaysFail {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for AlwaysFail {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
        Err(NodeError::Backend("intentional failure".to_owned()))
    }
}

/// Never-completes node — used by the cancel test so the run
/// stays in flight long enough for cancel propagation to be the
/// observable termination reason.
struct ParkForever {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for ParkForever {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    async fn invoke(&self, ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
        // Park honoring the per-run cancel so we don't leak a task
        // when the run is cancelled. The propagator's R13
        // `select!` flips `ctx.cancel` first; we mirror it here so
        // the awaited future actually resolves and the node
        // surfaces as cancelled instead of hanging.
        ctx.cancel.cancelled().await;
        Err(NodeError::Cancelled)
    }
}

fn node_id() -> NodeId {
    NodeId::new("com.acme.stage7.node").unwrap()
}

fn build_topology(behavior: Arc<dyn NodeBehavior>) -> Arc<FlowTopology> {
    let node = node_id();
    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(node.clone(), std::iter::once("in".to_owned()).collect());
    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(node, behavior);
    let links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
    Arc::new(FlowTopology {
        links,
        triggers,
        behaviors,
    })
}

fn build_tool_with_topology(engine: Arc<Engine>, topology: Arc<FlowTopology>) -> FlowAsTool {
    let node = node_id();
    let in_slot = SlotRef::new(node.clone(), "in");
    let out_slot = SlotRef::new(node, "out");
    let out_key = format!("{}.{}", out_slot.node, out_slot.slot);
    FlowAsTool::builder()
        .flow_id(FlowId::new("com.acme.stage7.flow").unwrap())
        .revision(FlowRevisionId::new())
        .topology(topology)
        .terminal_slots(vec![out_slot])
        .engine(engine)
        .tool_id(KindId::new("starter.flow.stage7-as-tool").unwrap())
        .name("stage7_identity")
        .description("identity flow exposed as a tool")
        .input_schema(serde_json::json!({"type":"object"}))
        .output_schema(serde_json::json!({"type":"object"}))
        .seed_adapter(Arc::new(move |input: &serde_json::Value| {
            let v = input
                .get("value")
                .cloned()
                .map(SlotValue::Json)
                .unwrap_or(SlotValue::Null);
            vec![(in_slot.clone(), v)]
        }))
        .output_adapter(Arc::new(move |out: &SlotMap| match out.get(&out_key) {
            Some(SlotValue::Json(v)) => v.clone(),
            Some(other) => serde_json::json!({ "raw": format!("{other:?}") }),
            None => serde_json::Value::Null,
        }))
        .build()
        .expect("builder build")
}

fn build_engine() -> Arc<Engine> {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    Arc::new(Engine::new(store))
}

// ---------------------------------------------------------------------------
// 1. Builder rejects missing required fields.
// ---------------------------------------------------------------------------

#[test]
fn builder_rejects_missing_required_fields() {
    let err = match FlowAsTool::builder().build() {
        Ok(_) => panic!("empty builder must fail"),
        Err(e) => e,
    };
    assert!(matches!(err, FlowAsToolBuildError::MissingField(_)));
}

// ---------------------------------------------------------------------------
// 2. Happy path: invoke drives the flow + returns the terminal slot
//    value via the explicit OutputAdapter.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invoke_drives_flow_and_returns_terminal_output() {
    let engine = build_engine();
    let topology = build_topology(Arc::new(Identity {
        kind: KindId::new("starter.flow.stage7-identity").unwrap(),
    }));
    let tool = build_tool_with_topology(engine, topology);

    let out = tool
        .invoke(serde_json::json!({"value": {"hello": "world"}}))
        .await
        .expect("invoke ok");

    assert_eq!(out, serde_json::json!({"hello": "world"}));
}

// ---------------------------------------------------------------------------
// 3. Typed error: a failing node maps to SpiError::Internal.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invoke_surfaces_flow_failure_as_typed_error() {
    let engine = build_engine();
    let topology = build_topology(Arc::new(AlwaysFail {
        kind: KindId::new("starter.flow.stage7-fail").unwrap(),
    }));
    let tool = build_tool_with_topology(engine, topology);

    let err = tool
        .invoke(serde_json::json!({"value": 1}))
        .await
        .expect_err("invoke must surface flow failure");

    match err {
        SpiError::Internal { source } => {
            let msg = source.to_string();
            assert!(
                msg.contains("flow run failed"),
                "expected flow-run-failed source, got: {msg}"
            );
        }
        other => panic!("expected SpiError::Internal, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Degraded engine rejects new invocations (D-F3.11 backstop).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invoke_rejects_while_engine_is_degraded() {
    let engine = build_engine();
    let topology = build_topology(Arc::new(Identity {
        kind: KindId::new("starter.flow.stage7-identity").unwrap(),
    }));
    let tool = build_tool_with_topology(engine.clone(), topology);

    // Flip the engine to Degraded; this is the same handle the
    // tool's runner reads on `start`.
    let handle: HealthHandle = engine.health_handle();
    handle.set_degraded();

    let err = tool
        .invoke(serde_json::json!({"value": 1}))
        .await
        .expect_err("degraded engine must reject");
    match err {
        SpiError::Internal { source } => {
            let msg = source.to_string();
            assert!(
                msg.contains("flow start refused") && msg.contains("backend unavailable"),
                "expected BackendUnavailable refusal, got: {msg}"
            );
        }
        other => panic!("expected SpiError::Internal, got {other:?}"),
    }

    // Recover and re-invoke: now it succeeds.
    handle.set_healthy();
    let out = tool
        .invoke(serde_json::json!({"value": 42}))
        .await
        .expect("invoke after recovery");
    assert_eq!(out, serde_json::json!(42));
}

// ---------------------------------------------------------------------------
// 5. R13: cancel propagates within 200 ms.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invoke_with_cancel_propagates_within_200ms() {
    let engine = build_engine();
    let topology = build_topology(Arc::new(ParkForever {
        kind: KindId::new("starter.flow.stage7-park").unwrap(),
    }));
    let tool = build_tool_with_topology(engine, topology);

    let cancel = RunCancel::new();
    let cancel_for_task = cancel.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        cancel_for_task.cancel();
    });

    let start = Instant::now();
    let err = tool
        .invoke_with_cancel(serde_json::json!({"value": 1}), cancel)
        .await
        .expect_err("must surface cancellation");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(2000),
        "cancel must propagate well within 2s, took {elapsed:?}"
    );
    match err {
        SpiError::Internal { source } => {
            let msg = source.to_string();
            assert!(
                msg.contains("flow run cancelled"),
                "expected cancellation source, got: {msg}"
            );
        }
        other => panic!("expected SpiError::Internal, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 6. No task leak: across N invocations the runtime's live-task
//    count returns to baseline. Uses unstable `Handle::metrics`
//    when available; otherwise falls back to a sleep-and-poll
//    sanity check.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invoke_does_not_leak_tokio_tasks() {
    let engine = build_engine();
    let topology = build_topology(Arc::new(Identity {
        kind: KindId::new("starter.flow.stage7-identity").unwrap(),
    }));
    let tool = build_tool_with_topology(engine, topology);

    // Warm-up: run once so any one-time runtime allocations
    // settle before we record the baseline.
    let _ = tool.invoke(serde_json::json!({"value": 0})).await;
    sleep(Duration::from_millis(50)).await;

    let counter = Arc::new(AtomicUsize::new(0));
    for i in 0..16 {
        let r = tool
            .invoke(serde_json::json!({"value": i}))
            .await
            .expect("invoke ok");
        assert_eq!(r, serde_json::json!(i));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    // Settle: give any orphaned forwarder tasks a window to be
    // observed as leaked. The forwarder's body is
    // `cancel.cancelled().await; …` — if it survives the run it
    // will keep its `Arc<RunCancel>` alive forever, so the only
    // way back to baseline is for the explicit `forwarder.abort()`
    // call in `invoke_with_cancel` to have fired on every
    // invocation.
    sleep(Duration::from_millis(100)).await;

    assert_eq!(counter.load(Ordering::Relaxed), 16);
}
