//! SCOPE Smoke test: **Engine is reader of policies** (R3 + R12).
//!
//! Two assertions together prove the engine never hardcodes policy
//! semantics:
//!
//! 1. **Engine.stop drives declared safe-states** — build an engine
//!    with two [`WritableOutput`]s, one whose declared `safe_state` is
//!    `fail-safe(Int(0))` and one whose declared `safe_state` is
//!    semantically `hold-last` (expressed as the value the output
//!    holds at stop time — the smoke fake returns its current held
//!    value from `safe_state()`). Subscribe to the [`GraphStore`]
//!    before [`Engine::stop`]; assert each output receives exactly one
//!    engine-driven write whose value matches its declared
//!    safe-state.
//!
//! 2. **No hardcoded policy-slot match arms** — walk
//!    `crates/starter-flow/src/` at test time and reject any line
//!    matching `^\s*"safe_state"`, `^\s*"session_policy"`,
//!    `^\s*"on_failure"`, `^\s*"cost_cap"`, `^\s*"trigger"`,
//!    `^\s*"auth"`, or `^\s*"timeout"` — the classic `match
//!    slot.name() { "safe_state" => ... }` anti-pattern that would
//!    make the engine a *reader plus interpreter* of one specific
//!    schema. Doc-comment hits (`//`, `///`, `//!`) are skipped.
//!
//! This is the **SCOPE-Smoke** grep test, distinct from the R3
//! contract test (`r3_no_policy_match_arms.rs`) — the SCOPE smoke
//! lives next to the safe-state walk assertion so a single test file
//! tells the whole "engine is a reader" story; the R3 contract test
//! lands separately for blast-radius granularity.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::Mutex;
use tokio::time::timeout;

use async_trait::async_trait;
use starter_flow_spi::graph::{GraphError, GraphStore, SubscribeOpts, WriteSlotOpts};
use starter_flow_spi::node::{NodeId, SlotRef, SlotValue};

use starter_flow::engine::{Engine, WritableOutput};
use starter_flow::graph::InMemoryGraphStore;

// ---------------------------------------------------------------------------
// Fake writable outputs.
// ---------------------------------------------------------------------------

/// `fail-safe(value)` flavour: returns a fixed safe value.
struct FailSafeOutput {
    slot: SlotRef,
    safe: SlotValue,
    writes: Arc<AtomicUsize>,
}

#[async_trait]
impl WritableOutput for FailSafeOutput {
    fn slot(&self) -> SlotRef {
        self.slot.clone()
    }
    fn safe_state(&self) -> SlotValue {
        self.safe.clone()
    }
    async fn write_safe_state(&self, store: &dyn GraphStore) -> Result<(), GraphError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        store
            .write_slot(&self.slot(), self.safe_state(), WriteSlotOpts::live())
            .await
    }
}

/// `hold-last` flavour: `safe_state()` returns whatever value the
/// output currently holds. The smoke writes a live value before
/// `Engine::stop`, then expects the stop walk to drive that same
/// value through `write_slot` (forced, since R3 idempotency would
/// otherwise swallow the equal-value re-write — the engine's
/// safe-state walk uses the default trait `write_safe_state` which
/// goes through `WriteSlotOpts::live()`; the test uses a *different*
/// live value than the previously-held one so the SlotChanged event
/// fires).
struct HoldLastOutput {
    slot: SlotRef,
    last: Mutex<SlotValue>,
    writes: Arc<AtomicUsize>,
}

#[async_trait]
impl WritableOutput for HoldLastOutput {
    fn slot(&self) -> SlotRef {
        self.slot.clone()
    }
    fn safe_state(&self) -> SlotValue {
        // `safe_state` is sync per the trait; use `try_lock` and
        // fall back to a default if contended. The smoke holds no
        // concurrent borrow on `last`, so this never falls back.
        match self.last.try_lock() {
            Ok(g) => g.clone(),
            Err(_) => SlotValue::Null,
        }
    }
    async fn write_safe_state(&self, store: &dyn GraphStore) -> Result<(), GraphError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        let v = self.last.lock().await.clone();
        store
            .write_slot(&self.slot(), v, WriteSlotOpts::live())
            .await
    }
}

// ---------------------------------------------------------------------------
// Assertion 1: Engine.stop walks writables and writes declared safe
// states through the GraphStore chokepoint (R3 + R12).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn engine_stop_drives_declared_safe_states() {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Engine::new(store.clone());

    let slot_a = SlotRef::new(NodeId::new("flow.smoke.fail_safe").unwrap(), "value");
    let slot_b = SlotRef::new(NodeId::new("flow.smoke.hold_last").unwrap(), "value");

    // Seed `slot_b` so its hold-last "held value" is distinct from
    // its safe-state drive — that lets us assert a SlotChanged
    // envelope fires for the hold-last output too.
    store
        .write_slot(&slot_b, SlotValue::Int(7), WriteSlotOpts::live())
        .await
        .unwrap();

    let writes_a = Arc::new(AtomicUsize::new(0));
    let writes_b = Arc::new(AtomicUsize::new(0));

    let fail_safe = Arc::new(FailSafeOutput {
        slot: slot_a.clone(),
        safe: SlotValue::Int(0),
        writes: writes_a.clone(),
    });
    let hold_last = Arc::new(HoldLastOutput {
        slot: slot_b.clone(),
        // The hold-last value at stop time — chosen distinct from
        // the seed so the engine-driven write produces a SlotChanged.
        last: Mutex::new(SlotValue::Int(42)),
        writes: writes_b.clone(),
    });

    engine.register_writable(fail_safe).await;
    engine.register_writable(hold_last).await;

    // Subscribe to the store BEFORE stop — the smoke is "engine is
    // *observed* writing safe-states through the chokepoint".
    let mut sub = store.subscribe(SubscribeOpts::default());

    engine.start().await.unwrap();
    engine.stop().await.unwrap();

    // Each writable was invoked exactly once.
    assert_eq!(writes_a.load(Ordering::SeqCst), 1, "fail-safe write count");
    assert_eq!(writes_b.load(Ordering::SeqCst), 1, "hold-last write count");

    // Collect SlotChanged envelopes on the two writable slots.
    let mut saw_a: Option<SlotValue> = None;
    let mut saw_b: Option<SlotValue> = None;
    while let Ok(Some(env)) = timeout(Duration::from_millis(100), sub.next()).await {
        if env.slot == slot_a {
            assert!(saw_a.is_none(), "fail-safe slot saw more than one event");
            saw_a = env.value;
        } else if env.slot == slot_b {
            assert!(saw_b.is_none(), "hold-last slot saw more than one event");
            saw_b = env.value;
        }
        if saw_a.is_some() && saw_b.is_some() {
            break;
        }
    }

    assert_eq!(
        saw_a,
        Some(SlotValue::Int(0)),
        "fail-safe(0) safe-state envelope",
    );
    assert_eq!(
        saw_b,
        Some(SlotValue::Int(42)),
        "hold-last safe-state envelope reflects the value held at stop",
    );

    // Store reflects the safe values after stop.
    assert_eq!(store.read_slot(&slot_a).await.unwrap(), SlotValue::Int(0));
    assert_eq!(store.read_slot(&slot_b).await.unwrap(), SlotValue::Int(42));
}

// ---------------------------------------------------------------------------
// Assertion 2: SCOPE-Smoke grep — no hardcoded policy-slot match
// arms in starter-flow's own src/ tree.
// ---------------------------------------------------------------------------

/// Policy-slot names that, if pattern-matched as bare string literals
/// at column-leading whitespace, indicate the engine has grown a
/// hardcoded reader of one specific schema.
///
/// Lifted from the SCOPE "Engine is reader of policies" smoke and
/// the broader R3 policy-slot surface (`safe_state`, `session_policy`,
/// `on_failure`, `cost_cap`, `trigger`, `auth`, `timeout`).
const POLICY_SLOT_NAMES: &[&str] = &[
    "safe_state",
    "session_policy",
    "on_failure",
    "cost_cap",
    "trigger",
    "auth",
    "timeout",
];

fn walk_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_hardcoded_policy_slot_match_arms_in_src() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_dir = Path::new(manifest_dir).join("src");
    assert!(
        src_dir.is_dir(),
        "starter-flow src directory missing at {src_dir:?}"
    );

    let mut files = Vec::new();
    walk_rs_files(&src_dir, &mut files);
    assert!(!files.is_empty(), "no .rs files found under {src_dir:?}");

    let mut violations: Vec<String> = Vec::new();

    for path in files {
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            // Skip doc comments and ordinary line comments.
            if trimmed.starts_with("//") {
                continue;
            }
            // Match the `^\s*"<policy>"` shape — the canonical
            // pattern for a hardcoded `match slot.name() {
            // "safe_state" => ... }` arm.
            for &name in POLICY_SLOT_NAMES {
                let needle = format!("\"{name}\"");
                if trimmed.starts_with(&needle) {
                    violations.push(format!(
                        "{}:{}: hardcoded policy-slot literal {:?}\n    {}",
                        path.display(),
                        idx + 1,
                        name,
                        line,
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "starter-flow/src contains hardcoded policy-slot match arms — the engine has stopped being a *reader* of policies:\n{}",
        violations.join("\n"),
    );
}
