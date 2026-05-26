//! `sleep` — timed delay node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "Relationship to
//! existing crates" (the `starter-flow-nodes` row lists `sleep`
//! alongside the other built-ins) and scheduled in § "Phase 5 —
//! Remaining built-in node kinds". Holds propagation for a declared
//! duration then forwards its input to its output. Cancellation
//! interrupts the wait per § "R13 — Streaming, cancellation,
//! observability reuse existing seams".
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** `sleep` is a [`NodeBehavior`]
//!   impl, nothing more.
//! - **R2 — One write chokepoint.** The body returns its output
//!   [`SlotMap`]; the propagator funnels it through
//!   `GraphStore::write_slot`.
//! - **R5 — Stateless behaviours.** Unit struct; no per-invocation
//!   state.
//! - **R10 — Reverse-DNS ids.** [`KIND_ID`] is verbatim under
//!   `starter.flow.*`.
//! - **R12 observability.** Every invocation opens a `sleep.invoke`
//!   tracing span recording `(node_id, run_id, duration_ms,
//!   cancel_observed)`.
//! - **R13 cancellation.** The wait is `tokio::select!`ed against
//!   `ctx.cancel.cancelled()`; a cancelled run aborts the wait
//!   immediately and surfaces [`NodeError::Cancelled`].

use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

use starter_flow_spi::node::{
    anyhow_compat, KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.sleep";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub static DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.sleep.label",
        "starter.flow.node.sleep.summary",
        "starter.flow.node.sleep.help",
    );

/// Mandatory input slot carrying the wait duration in milliseconds.
/// Must be a non-negative [`SlotValue::Int`].
pub const DURATION_MS_SLOT: &str = "duration_ms";

/// Optional input slot whose value is forwarded to [`OUT_SLOT`] after
/// the wait completes. Absent → the output map carries no `out`
/// entry (chained nodes treat that as "no payload").
pub const VALUE_SLOT: &str = "value";

/// Output slot carrying the passthrough copy of [`VALUE_SLOT`].
pub const OUT_SLOT: &str = "out";

/// Upper bound on the configurable wait. Held deliberately low (one
/// hour) so a typo in `duration_ms` can't permanently wedge a run.
/// Longer waits are a `trigger.schedule` job, not a `sleep`.
pub const MAX_DURATION_MS: i64 = 60 * 60 * 1000;

/// Typed errors surfaced by [`Sleep::invoke`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SleepError {
    /// The input did not carry a [`DURATION_MS_SLOT`] entry.
    #[error("sleep input missing `{DURATION_MS_SLOT}` slot")]
    MissingDuration,

    /// `duration_ms` was present but not a [`SlotValue::Int`].
    #[error("sleep `{DURATION_MS_SLOT}` must be SlotValue::Int milliseconds")]
    InvalidDurationType,

    /// `duration_ms` was negative or exceeded [`MAX_DURATION_MS`].
    #[error("sleep `{DURATION_MS_SLOT}` out of range: {0} (must be 0..={MAX_DURATION_MS})")]
    DurationOutOfRange(i64),
}

impl SleepError {
    fn into_node_error(self) -> NodeError {
        NodeError::Other(anyhow_compat::Error(Box::new(self)))
    }
}

/// `sleep` node-kind behaviour. Stateless (R5) — unit struct.
pub struct Sleep {
    kind: KindId,
}

impl Default for Sleep {
    fn default() -> Self {
        Self::new()
    }
}

impl Sleep {
    /// Construct a [`Sleep`] node body. Panics if [`KIND_ID`] is not a
    /// valid reverse-DNS identifier (compile-time invariant).
    pub fn new() -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
        }
    }
}

#[async_trait]
impl NodeBehavior for Sleep {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    fn trigger_slots(&self) -> &'static [&'static str] {
        &[VALUE_SLOT]
    }

    fn read_slots(&self) -> &'static [&'static str] {
        &[DURATION_MS_SLOT]
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let duration_ms = match input.remove(DURATION_MS_SLOT) {
            None => return Err(SleepError::MissingDuration.into_node_error()),
            Some(SlotValue::Int(n)) => n,
            Some(_) => return Err(SleepError::InvalidDurationType.into_node_error()),
        };
        if !(0..=MAX_DURATION_MS).contains(&duration_ms) {
            return Err(SleepError::DurationOutOfRange(duration_ms).into_node_error());
        }

        let value = input.remove(VALUE_SLOT);

        let span = tracing::info_span!(
            "sleep.invoke",
            node_id = %ctx.node,
            run_id = %ctx.run,
            duration_ms = duration_ms,
            cancel_observed = tracing::field::Empty,
        );
        let _enter = span.enter();

        // Fast path: a zero-ms wait is a no-op; skip the select! so a
        // degenerate config doesn't pay an unnecessary yield.
        if duration_ms == 0 {
            span.record("cancel_observed", false);
        } else {
            let sleep = tokio::time::sleep(Duration::from_millis(duration_ms as u64));
            tokio::select! {
                () = sleep => {
                    span.record("cancel_observed", false);
                }
                () = ctx.cancel.cancelled() => {
                    span.record("cancel_observed", true);
                    return Err(NodeError::Cancelled);
                }
            }
        }

        let mut out = SlotMap::new();
        if let Some(v) = value {
            out.insert(OUT_SLOT.to_owned(), v);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    use starter_flow_spi::node::NodeId;
    use starter_flow_spi::Cancel;

    struct NoCancel;
    impl Cancel for NoCancel {
        fn is_cancelled(&self) -> bool {
            false
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(std::future::pending())
        }
    }

    struct FlagCancel {
        flag: Arc<AtomicBool>,
    }
    impl Cancel for FlagCancel {
        fn is_cancelled(&self) -> bool {
            self.flag.load(Ordering::SeqCst)
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            let flag = self.flag.clone();
            Box::pin(async move {
                while !flag.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            })
        }
    }

    fn make_ctx<'a>(node: &'a NodeId, cancel: &'a dyn Cancel) -> NodeCtx<'a> {
        NodeCtx::new(
            starter_flow_spi::flow::RunId::new(),
            node,
            cancel,
            starter_flow_spi::skill::SkillSelection::NONE,
            &starter_flow_spi::state::NOOP_NODE_STATE_STORE,
        )
    }

    #[tokio::test]
    async fn zero_ms_is_immediate_passthrough() {
        let body = Sleep::new();
        let node = NodeId::new("test.sleep").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = SlotMap::new();
        input.insert(DURATION_MS_SLOT.to_owned(), SlotValue::Int(0));
        input.insert(VALUE_SLOT.to_owned(), SlotValue::String("hello".into()));
        let started = Instant::now();
        let out = body.invoke(ctx, input).await.expect("invoke");
        assert!(started.elapsed() < Duration::from_millis(50));
        assert!(matches!(out.get(OUT_SLOT), Some(SlotValue::String(s)) if s == "hello"));
    }

    #[tokio::test]
    async fn small_wait_completes() {
        let body = Sleep::new();
        let node = NodeId::new("test.sleep").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = SlotMap::new();
        input.insert(DURATION_MS_SLOT.to_owned(), SlotValue::Int(20));
        input.insert(VALUE_SLOT.to_owned(), SlotValue::Bool(true));
        let started = Instant::now();
        let out = body.invoke(ctx, input).await.expect("invoke");
        let elapsed = started.elapsed();
        assert!(elapsed >= Duration::from_millis(15));
        assert!(elapsed < Duration::from_millis(500));
        assert!(matches!(out.get(OUT_SLOT), Some(SlotValue::Bool(true))));
    }

    #[tokio::test]
    async fn missing_value_emits_empty_output() {
        let body = Sleep::new();
        let node = NodeId::new("test.sleep").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = SlotMap::new();
        input.insert(DURATION_MS_SLOT.to_owned(), SlotValue::Int(0));
        let out = body.invoke(ctx, input).await.expect("invoke");
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn cancel_aborts_wait() {
        let body = Sleep::new();
        let node = NodeId::new("test.sleep").unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        let cancel = FlagCancel { flag: flag.clone() };
        let ctx = make_ctx(&node, &cancel);
        let mut input = SlotMap::new();
        // 10s wait that we'll cancel after ~20ms.
        input.insert(DURATION_MS_SLOT.to_owned(), SlotValue::Int(10_000));
        input.insert(VALUE_SLOT.to_owned(), SlotValue::Null);

        let cancel_flag = flag.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            cancel_flag.store(true, Ordering::SeqCst);
        });

        let started = Instant::now();
        let err = body
            .invoke(ctx, input)
            .await
            .expect_err("must be cancelled");
        assert!(matches!(err, NodeError::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn missing_duration_is_error() {
        let body = Sleep::new();
        let node = NodeId::new("test.sleep").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let err = body
            .invoke(ctx, SlotMap::new())
            .await
            .expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains(DURATION_MS_SLOT), "{msg}");
    }

    #[tokio::test]
    async fn negative_duration_is_error() {
        let body = Sleep::new();
        let node = NodeId::new("test.sleep").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = SlotMap::new();
        input.insert(DURATION_MS_SLOT.to_owned(), SlotValue::Int(-5));
        let err = body.invoke(ctx, input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains("out of range"), "{msg}");
    }

    #[tokio::test]
    async fn wrong_duration_type_is_error() {
        let body = Sleep::new();
        let node = NodeId::new("test.sleep").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = SlotMap::new();
        input.insert(DURATION_MS_SLOT.to_owned(), SlotValue::String("100".into()));
        let err = body.invoke(ctx, input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains("SlotValue::Int"), "{msg}");
    }
}
