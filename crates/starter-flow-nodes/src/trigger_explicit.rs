//! `trigger.explicit` — manually-fired entry node.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R3 — The engine
//! is a reader of policies, never an owner" (the `trigger` policy
//! variant `explicit`) and § "R1 — Everything is a Node" ("A trigger
//! (explicit, event-driven, scheduled, webhook) is a node"). The
//! explicit form fires only when an operator or API caller invokes
//! the flow directly. Scheduled in § "Phase 5 — Remaining built-in
//! node kinds" and locked at D-F5.1 of the
//! `starter-flow-phase5-demo` job.
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** `trigger.explicit` is just a
//!   `NodeBehavior` impl whose `invoke` blocks until a host-side
//!   `Sender<SlotMap>` delivers a payload.
//! - **R2 — One write chokepoint.** The body returns its output
//!   [`SlotMap`]; the propagator funnels every value through
//!   `GraphStore::write_slot`. No direct slot writes.
//! - **R5 — Stateless behaviours.** The body holds `Arc<dyn
//!   TriggerChannelRegistry>`; the host-side fire channel is the
//!   substrate, not invocation state.
//! - **R10 — Reverse-DNS ids.** [`KIND_ID`] is verbatim under
//!   `starter.flow.*`; channel ids are validated as [`KindId`]
//!   reverse-DNS strings.
//! - **R12 observability.** Every invocation opens a
//!   `trigger_explicit.invoke` tracing span recording `(node_id,
//!   run_id, channel_id, cancel_observed)`.
//! - **R13 cancellation.** The body `tokio::select!`s the channel
//!   receive against `ctx.cancel.cancelled()`; cancel-to-exit is
//!   bounded by the channel's `Mutex` acquire (sub-millisecond in
//!   practice).

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use async_trait::async_trait;
use schemars::{schema::RootSchema, JsonSchema};
use serde::Deserialize;
use thiserror::Error;
use tracing::Instrument;

use starter_flow_spi::node::{
    anyhow_compat, KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.trigger.explicit";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub static DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.trigger-explicit.label",
        "starter.flow.node.trigger-explicit.summary",
        "starter.flow.node.trigger-explicit.help",
    );

/// Mandatory config slot carrying the reverse-DNS [`KindId`] of the
/// fire channel the host bound this trigger to. The slot value must
/// be a [`SlotValue::String`]; the body validates it through
/// [`KindId::new`].
pub const CHANNEL_ID_SLOT: &str = "channel_id";

/// Output slot the body writes the received payload [`SlotMap`] into
/// as [`SlotValue::Json`]. Downstream nodes read the payload as a
/// JSON value the host produced when calling [`TriggerSender::fire`].
pub const PAYLOAD_SLOT: &str = "payload";

/// Publish-time configuration carried on a `trigger.explicit` node's
/// `settings:` field in a flow body. Per
/// [`DOCS/flow/scope/settings.md`](../../../DOCS/flow/scope/settings.md)
/// Phase S-4.
///
/// Runtime [`TriggerExplicit::invoke`] still reads
/// [`CHANNEL_ID_SLOT`] from the input [`SlotMap`]; once
/// `TopologyResolver::resolve` lands (`DOCS/flow/scope/hot-reload.md`
/// HR5) it will project [`Self::channel_id`] into that slot. Until
/// then this struct only powers schema-fetch surfaces.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TriggerExplicitSettings {
    /// Reverse-DNS id of the fire channel the host bound this
    /// trigger to (e.g. `examples.notes.demo`). The body resolves
    /// this against the [`TriggerChannelRegistry`] at invoke time.
    pub channel_id: String,
}

/// Derived JSON Schema for [`TriggerExplicitSettings`]. Returned by
/// reference from [`TriggerExplicit::config_schema`]; built once per
/// process via [`LazyLock`].
pub static TRIGGER_EXPLICIT_SETTINGS_SCHEMA: LazyLock<RootSchema> =
    LazyLock::new(|| schemars::schema_for!(TriggerExplicitSettings));

/// Receiver half of a host-bound trigger channel.
///
/// Wraps an `Arc<tokio::sync::Mutex<...>>` so multiple successive
/// invocations of the same trigger across runs share one channel —
/// the `Mutex` serialises consumers; the `Arc` lets the registry hand
/// out clones without consuming the receiver.
#[derive(Clone)]
pub struct TriggerReceiver {
    inner: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<serde_json::Value>>>,
}

/// Sender half of a host-bound trigger channel. The host calls
/// [`Self::fire`] to wake a blocked `trigger.explicit` invocation
/// with a JSON payload.
#[derive(Clone)]
pub struct TriggerSender {
    inner: tokio::sync::mpsc::Sender<serde_json::Value>,
}

impl TriggerSender {
    /// Wake the next blocked `trigger.explicit` invocation on this
    /// channel with the given payload. Returns `Err` only if no
    /// receiver is alive (i.e. the registry was dropped).
    pub async fn fire(
        &self,
        payload: serde_json::Value,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<serde_json::Value>> {
        self.inner.send(payload).await
    }
}

/// Host-controlled registry of fire channels the `trigger.explicit`
/// body resolves `channel_id` against. R5: the body holds an
/// `Arc<dyn TriggerChannelRegistry>` handed in at construction time;
/// the registry's surface is read-only from the body's perspective.
pub trait TriggerChannelRegistry: Send + Sync + 'static {
    /// Look up a channel's receiver by its reverse-DNS [`KindId`].
    /// Returns `None` if no channel is registered under that id.
    fn lookup(&self, channel_id: &KindId) -> Option<TriggerReceiver>;
}

/// In-memory [`TriggerChannelRegistry`] populated at engine-build
/// time. The host calls [`Self::bind`] for each explicit-trigger
/// channel it wants to expose; `bind` creates an `mpsc::channel`,
/// stores the receiver, and returns the matching [`TriggerSender`]
/// the host keeps for the lifetime of the engine.
#[derive(Default)]
pub struct StaticTriggerChannelRegistry {
    channels: HashMap<KindId, TriggerReceiver>,
}

impl StaticTriggerChannelRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a fire channel under `channel_id` and return the
    /// [`TriggerSender`] the host calls to fire the trigger. The
    /// buffer is the bounded mpsc capacity; back-pressure is the
    /// host's concern.
    pub fn bind(&mut self, channel_id: KindId, buffer: usize) -> TriggerSender {
        let (tx, rx) = tokio::sync::mpsc::channel(buffer.max(1));
        self.channels.insert(
            channel_id,
            TriggerReceiver {
                inner: Arc::new(tokio::sync::Mutex::new(rx)),
            },
        );
        TriggerSender { inner: tx }
    }
}

impl TriggerChannelRegistry for StaticTriggerChannelRegistry {
    fn lookup(&self, channel_id: &KindId) -> Option<TriggerReceiver> {
        self.channels.get(channel_id).cloned()
    }
}

/// Typed errors surfaced by [`TriggerExplicit::invoke`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TriggerExplicitError {
    /// The input did not carry a string-valued [`CHANNEL_ID_SLOT`].
    #[error(
        "trigger.explicit input missing `{CHANNEL_ID_SLOT}` slot \
         (must be SlotValue::String naming the channel's KindId)"
    )]
    MissingChannelId,

    /// The `channel_id` was a string but not a valid reverse-DNS
    /// [`KindId`].
    #[error("trigger.explicit `channel_id` is not a valid reverse-DNS KindId: {0}")]
    InvalidChannelId(String),

    /// The `channel_id` parsed but no channel is registered under it.
    #[error("trigger.explicit channel not registered: {0}")]
    UnregisteredChannel(KindId),

    /// The fire channel was closed before any payload was delivered
    /// (i.e. every [`TriggerSender`] was dropped). The invocation
    /// surfaces this as a domain error rather than blocking forever.
    #[error("trigger.explicit channel `{0}` closed without a payload")]
    ChannelClosed(KindId),
}

impl TriggerExplicitError {
    fn into_node_error(self) -> NodeError {
        NodeError::Other(anyhow_compat::Error(Box::new(self)))
    }
}

/// `trigger.explicit` node-kind behaviour. Stateless (R5).
pub struct TriggerExplicit {
    kind: KindId,
    registry: Arc<dyn TriggerChannelRegistry>,
}

impl TriggerExplicit {
    /// Construct a [`TriggerExplicit`] backed by the given channel
    /// registry. Panics if [`KIND_ID`] is not a valid reverse-DNS
    /// identifier (compile-time-checkable invariant of this crate).
    pub fn new(registry: Arc<dyn TriggerChannelRegistry>) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
            registry,
        }
    }
}

#[async_trait]
impl NodeBehavior for TriggerExplicit {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    fn trigger_slots(&self) -> &'static [&'static str] {
        &[PAYLOAD_SLOT]
    }

    fn read_slots(&self) -> &'static [&'static str] {
        &[CHANNEL_ID_SLOT]
    }

    fn config_schema(&self) -> &'static RootSchema {
        &TRIGGER_EXPLICIT_SETTINGS_SCHEMA
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        // Pull `channel_id` (validated as a KindId per R10 reverse-DNS).
        let channel_id_raw = match input.remove(CHANNEL_ID_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => return Err(TriggerExplicitError::MissingChannelId.into_node_error()),
        };
        let channel_id = KindId::new(channel_id_raw.clone()).map_err(|_| {
            TriggerExplicitError::InvalidChannelId(channel_id_raw).into_node_error()
        })?;

        // R12 observability span.
        let span = tracing::info_span!(
            "trigger_explicit.invoke",
            node_id = %ctx.node,
            run_id = %ctx.run,
            channel_id = %channel_id,
            cancel_observed = tracing::field::Empty,
        );
        // `Instrument` (not `span.enter()`) — the body awaits a
        // mutex acquire and a channel receive. A span guard
        // across `.await` corrupts the thread-local span stack
        // when the future migrates between tokio workers and
        // later panics `tracing-subscriber` on an unrelated
        // emit.
        let span_for_record = span.clone();
        async move {
            let channel = match self.registry.lookup(&channel_id) {
                Some(c) => c,
                None => {
                    tracing::warn!(channel_id = %channel_id, "channel not registered");
                    span_for_record.record("cancel_observed", false);
                    return Err(TriggerExplicitError::UnregisteredChannel(channel_id).into_node_error());
                }
            };

            // R13 cancellation. Race the channel receive against
            // `ctx.cancel.cancelled()` inside a `select!`. The receiver
            // is behind a Mutex so concurrent invocations on the same
            // channel serialise; the Mutex acquire is sub-millisecond in
            // practice.
            let cancelled = ctx.cancel.cancelled();
            tokio::pin!(cancelled);
            let mut rx = channel.inner.lock().await;

            let payload = tokio::select! {
                biased;
                _ = &mut cancelled => {
                    span_for_record.record("cancel_observed", true);
                    tracing::info!(channel_id = %channel_id, "trigger.explicit cancelled before payload");
                    return Err(NodeError::Cancelled);
                }
                recv = rx.recv() => match recv {
                    Some(p) => p,
                    None => {
                        span_for_record.record("cancel_observed", false);
                        tracing::warn!(channel_id = %channel_id, "trigger.explicit channel closed");
                        return Err(
                            TriggerExplicitError::ChannelClosed(channel_id).into_node_error()
                        );
                    }
                },
            };
            span_for_record.record("cancel_observed", false);

            let mut out = SlotMap::new();
            out.insert(PAYLOAD_SLOT.to_owned(), SlotValue::Json(payload));
            Ok(out)
        }
        .instrument(span)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use starter_flow_spi::node::{NodeId, SlotMap, SlotValue};
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
        notify: tokio::sync::Notify,
        flag: AtomicBool,
    }
    impl FlagCancel {
        fn new() -> Self {
            Self {
                notify: tokio::sync::Notify::new(),
                flag: AtomicBool::new(false),
            }
        }
        fn fire(&self) {
            self.flag.store(true, Ordering::SeqCst);
            self.notify.notify_waiters();
        }
    }
    impl Cancel for FlagCancel {
        fn is_cancelled(&self) -> bool {
            self.flag.load(Ordering::SeqCst)
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(async move {
                if self.flag.load(Ordering::SeqCst) {
                    return;
                }
                self.notify.notified().await;
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

    fn channel_id(s: &str) -> KindId {
        KindId::new(s).unwrap()
    }

    fn input_with(channel: &str) -> SlotMap {
        let mut m = SlotMap::new();
        m.insert(
            CHANNEL_ID_SLOT.to_owned(),
            SlotValue::String(channel.to_owned()),
        );
        m
    }

    /// Happy path: a fired payload arrives on the `payload` output
    /// slot verbatim.
    #[tokio::test]
    async fn fired_payload_propagates_to_output_slot() {
        let mut registry = StaticTriggerChannelRegistry::new();
        let sender = registry.bind(channel_id("examples.notes.demo"), 1);
        let node_kind = TriggerExplicit::new(Arc::new(registry));

        let node = NodeId::new("flow.test.te").unwrap();
        let cancel = NoCancel;
        let payload = serde_json::json!({"note_id": 42});
        sender.fire(payload.clone()).await.unwrap();

        let out = node_kind
            .invoke(make_ctx(&node, &cancel), input_with("examples.notes.demo"))
            .await
            .expect("happy path must succeed");

        assert_eq!(out.len(), 1);
        match out.get(PAYLOAD_SLOT) {
            Some(SlotValue::Json(v)) => assert_eq!(v, &payload),
            other => panic!("expected SlotValue::Json on `payload`; got {other:?}"),
        }
    }

    /// Missing `channel_id` slot returns
    /// `TriggerExplicitError::MissingChannelId`.
    #[tokio::test]
    async fn missing_channel_id_returns_typed_error() {
        let registry: Arc<dyn TriggerChannelRegistry> =
            Arc::new(StaticTriggerChannelRegistry::new());
        let node_kind = TriggerExplicit::new(registry);

        let node = NodeId::new("flow.test.te").unwrap();
        let cancel = NoCancel;
        let err = node_kind
            .invoke(make_ctx(&node, &cancel), SlotMap::new())
            .await
            .expect_err("missing channel_id must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other; got {err:?}");
        };
        let e = boxed
            .0
            .downcast::<TriggerExplicitError>()
            .expect("Other must wrap TriggerExplicitError");
        assert!(
            matches!(*e, TriggerExplicitError::MissingChannelId),
            "expected MissingChannelId; got {e:?}",
        );
    }

    /// A `channel_id` that does not resolve in the registry surfaces
    /// as `TriggerExplicitError::UnregisteredChannel`.
    #[tokio::test]
    async fn unregistered_channel_returns_typed_error() {
        let registry: Arc<dyn TriggerChannelRegistry> =
            Arc::new(StaticTriggerChannelRegistry::new());
        let node_kind = TriggerExplicit::new(registry);

        let node = NodeId::new("flow.test.te").unwrap();
        let cancel = NoCancel;
        let err = node_kind
            .invoke(
                make_ctx(&node, &cancel),
                input_with("examples.notes.missing"),
            )
            .await
            .expect_err("unregistered channel must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other; got {err:?}");
        };
        let e = boxed
            .0
            .downcast::<TriggerExplicitError>()
            .expect("Other must wrap TriggerExplicitError");
        assert!(
            matches!(*e, TriggerExplicitError::UnregisteredChannel(ref id) if id.as_str() == "examples.notes.missing"),
            "expected UnregisteredChannel; got {e:?}",
        );
    }

    /// A `Cancel` firing while waiting for a payload aborts the
    /// invocation in bounded time and surfaces `NodeError::Cancelled`.
    #[tokio::test]
    async fn cancel_before_fire_aborts_within_bounded_time() {
        let mut registry = StaticTriggerChannelRegistry::new();
        let _sender = registry.bind(channel_id("examples.notes.demo"), 1);
        let node_kind = Arc::new(TriggerExplicit::new(Arc::new(registry)));

        let cancel = Arc::new(FlagCancel::new());
        let cancel_for_fire = cancel.clone();

        let invoke_handle = tokio::spawn({
            let node_kind = node_kind.clone();
            let cancel = cancel.clone();
            async move {
                let node = NodeId::new("flow.test.te").unwrap();
                let ctx = NodeCtx::new(
                    starter_flow_spi::flow::RunId::new(),
                    Box::leak(Box::new(node)),
                    cancel.as_ref(),
                    starter_flow_spi::skill::SkillSelection::NONE,
                    &starter_flow_spi::state::NOOP_NODE_STATE_STORE,
                );
                node_kind
                    .invoke(ctx, input_with("examples.notes.demo"))
                    .await
            }
        });

        // Let the invocation block on recv, then fire cancel.
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel_for_fire.fire();

        let result = tokio::time::timeout(Duration::from_millis(200), invoke_handle)
            .await
            .expect("cancel must abort the blocked recv within 200ms")
            .expect("invoke task must not panic");
        let err = result.expect_err("cancelled invocation must return Err");
        assert!(
            matches!(err, NodeError::Cancelled),
            "expected NodeError::Cancelled; got {err:?}",
        );
    }

    /// Channel closure (all senders dropped) before any payload
    /// surfaces `TriggerExplicitError::ChannelClosed`.
    #[tokio::test]
    async fn dropped_sender_surfaces_channel_closed() {
        let mut registry = StaticTriggerChannelRegistry::new();
        let sender = registry.bind(channel_id("examples.notes.demo"), 1);
        let node_kind = TriggerExplicit::new(Arc::new(registry));

        // Drop the only sender before invoke runs.
        drop(sender);

        let node = NodeId::new("flow.test.te").unwrap();
        let cancel = NoCancel;
        let err = node_kind
            .invoke(make_ctx(&node, &cancel), input_with("examples.notes.demo"))
            .await
            .expect_err("dropped sender must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other; got {err:?}");
        };
        let e = boxed
            .0
            .downcast::<TriggerExplicitError>()
            .expect("Other must wrap TriggerExplicitError");
        assert!(
            matches!(*e, TriggerExplicitError::ChannelClosed(ref id) if id.as_str() == "examples.notes.demo"),
            "expected ChannelClosed; got {e:?}",
        );
    }
}
