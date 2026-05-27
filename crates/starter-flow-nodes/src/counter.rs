//! `counter` — durable tick-counter node (first consumer of
//! [`starter_flow_spi::state::NodeStateStore`]).
//!
//! Semantics defined by `DOCS/flow/scope/node-state.md`. On every
//! invocation the node reads its persisted `count` (or
//! [`CounterSettings::initial`] if absent), computes
//! `next = current + step`, persists `next`, and emits `next` on the
//! `out` slot. With `reset_on_redeploy = true` the engine's
//! [`NodeBehavior::on_redeploy`] hook clears the persisted state on
//! [`EditKind::Settings`], [`EditKind::Topology`] or
//! [`EditKind::Both`] so the next tick starts from
//! [`CounterSettings::initial`] again.
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** A `NodeBehavior` impl.
//! - **R2 — One write chokepoint.** Body returns its `out` slot; the
//!   propagator writes it through `GraphStore::write_slot`. State
//!   reads/writes go through the single
//!   [`NodeStateStore`] seam.
//! - **R5 — Stateless behaviours.** No `&mut self`; durable state
//!   lives behind the `NodeStateStore` seam, never on the kind.
//! - **R10 — Reverse-DNS ids.** [`KIND_ID`] is `starter.flow.counter`.
//! - **R12 observability.** Every invocation opens a
//!   `counter.invoke` tracing span recording `(node_id, run_id,
//!   prior, next)`.
//! - **R13 cancellation.** Cancel observed before the state read so
//!   the node aborts promptly with [`NodeError::Cancelled`].

use std::sync::LazyLock;

use async_trait::async_trait;
use schemars::{schema::RootSchema, JsonSchema};
use serde::Deserialize;
use tracing::Instrument;

use starter_flow_spi::node::{
    EditKind, KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};
use starter_flow_spi::state::{NodeStateKey, NodeStateStore};

/// Reverse-DNS kind id under the reserved `starter.flow.*` namespace.
pub const KIND_ID: &str = "starter.flow.counter";

/// Static metadata for the catalog / discovery surface.
pub static DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.counter.label",
        "starter.flow.node.counter.summary",
        "starter.flow.node.counter.help",
    );

/// Optional input slot. Present-but-unused today; reserved so a
/// downstream graph can chain a trigger into the counter without the
/// propagator dropping the value.
pub const IN_SLOT: &str = "in";
/// Output slot the body writes the post-increment value into.
pub const OUT_SLOT: &str = "out";
/// Key under which the [`NodeStateStore`] persists the counter's
/// current value. Opaque to the store; one tag per node id.
pub const STATE_KEY: &str = "count";

/// Publish-time configuration carried on a `counter` node's
/// `settings:` field in a flow body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CounterSettings {
    /// Amount added to the persisted count on every invocation.
    /// Defaults to `1`.
    #[serde(default = "default_step")]
    pub step: i64,
    /// Value used when no `count` has been persisted yet. Defaults
    /// to `0` (so the first emit is `initial + step = 1` with the
    /// default `step`).
    #[serde(default)]
    pub initial: i64,
    /// When `true`, the [`NodeBehavior::on_redeploy`] hook clears the
    /// persisted `count` on any [`EditKind`] edit so the next
    /// invocation restarts from `initial`. Defaults to `false`
    /// (preserve across redeploy).
    #[serde(default)]
    pub reset_on_redeploy: bool,
}

const fn default_step() -> i64 {
    1
}

impl Default for CounterSettings {
    fn default() -> Self {
        Self {
            step: default_step(),
            initial: 0,
            reset_on_redeploy: false,
        }
    }
}

/// Derived JSON Schema for [`CounterSettings`].
pub static COUNTER_SETTINGS_SCHEMA: LazyLock<RootSchema> =
    LazyLock::new(|| schemars::schema_for!(CounterSettings));

/// `counter` node-kind body. Stateless per R5; per-instance state
/// lives behind the [`NodeStateStore`] seam on [`NodeCtx::state`].
pub struct Counter {
    kind: KindId,
    settings: CounterSettings,
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

impl Counter {
    /// Construct a [`Counter`] with default settings (step=1, initial=0,
    /// reset_on_redeploy=false).
    pub fn new() -> Self {
        Self::with_settings(CounterSettings::default())
    }

    /// Construct a [`Counter`] with explicit settings. Settings are
    /// frozen on the body at construction time; the engine constructs
    /// one body per node revision through the kind registry, so a
    /// settings edit produces a fresh body (matching R5: behaviours
    /// stay stateless across edits).
    pub fn with_settings(settings: CounterSettings) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
            settings,
        }
    }
}

fn state_key(ctx: &NodeCtx<'_>) -> Result<NodeStateKey, NodeError> {
    let flow = ctx.flow.ok_or_else(|| NodeError::Domain {
        code: "counter_requires_flow_ctx",
        message: "counter node requires NodeCtx::with_flow; engine wiring missing flow id"
            .to_owned(),
    })?;
    NodeStateKey::new(flow.clone(), ctx.node.clone(), STATE_KEY)
        .map_err(|e| NodeError::Backend(format!("counter: failed to build NodeStateKey: {e}")))
}

async fn read_current(
    store: &dyn NodeStateStore,
    key: &NodeStateKey,
) -> Result<Option<i64>, NodeError> {
    let value = store
        .get(key)
        .await
        .map_err(|e| NodeError::Backend(format!("counter: state get failed: {e}")))?;
    match value {
        None => Ok(None),
        Some(v) => {
            let s = std::str::from_utf8(&v.bytes)
                .map_err(|e| NodeError::Backend(format!("counter: state value not utf-8: {e}")))?;
            let n: i64 = s.parse().map_err(|e| {
                NodeError::Backend(format!("counter: state value not i64 (`{s}`): {e}"))
            })?;
            Ok(Some(n))
        }
    }
}

#[async_trait]
impl NodeBehavior for Counter {
    fn trigger_slots(&self) -> &'static [&'static str] {
        &[IN_SLOT]
    }

    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    fn config_schema(&self) -> &'static RootSchema {
        &COUNTER_SETTINGS_SCHEMA
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
        // R13: cancel observed before any state I/O.
        if ctx.cancel.is_cancelled() {
            return Err(NodeError::Cancelled);
        }

        let key = state_key(&ctx)?;
        let current = read_current(ctx.state, &key).await?;
        let prior = current.unwrap_or(self.settings.initial);
        let next = prior.saturating_add(self.settings.step);

        // R12 observability. The span must wrap the future via
        // `Instrument`, not `span.enter()`, because the body
        // contains an `.await` on `ctx.state.put`. Holding a
        // span guard across `.await` is undefined: the future
        // can suspend on one tokio worker thread and resume on
        // another, but the guard's thread-local span stack is
        // not. The corruption surfaces as the well-known
        // `tracing-subscriber` panic at `registry/sharded.rs`:
        // `tried to clone a span (Id(...)) that already closed`
        // — emitted from an *unrelated* later span when the
        // poisoned thread-local is next touched, which makes
        // the bug appear far from the actual cause. The
        // `Instrument` adapter manages enter/exit per poll
        // correctly across all workers.
        let span = tracing::info_span!(
            "counter.invoke",
            node_id = %ctx.node,
            run_id = %ctx.run,
            prior = prior,
            next = next,
        );

        async {
            ctx.state
                .put(&key, next.to_string().into_bytes())
                .await
                .map_err(|e| NodeError::Backend(format!("counter: state put failed: {e}")))?;

            let mut out = SlotMap::new();
            out.insert(OUT_SLOT.to_owned(), SlotValue::Int(next));
            Ok(out)
        }
        .instrument(span)
        .await
    }

    async fn on_redeploy(&self, ctx: NodeCtx<'_>, edit: EditKind) -> Result<(), NodeError> {
        if !self.settings.reset_on_redeploy {
            return Ok(());
        }
        // Reset on every reported edit kind; the enum currently lists
        // only Settings/Topology/Both — all three should clear state
        // per the stage spec.
        let _ = edit;
        let key = state_key(&ctx)?;
        ctx.state
            .delete(&key)
            .await
            .map_err(|e| NodeError::Backend(format!("counter: state delete failed: {e}")))?;
        Ok(())
    }
}
