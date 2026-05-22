//! `log` — structured-log emission node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "Relationship to
//! existing crates" (the `starter-flow-nodes` row lists `log`
//! alongside the other built-ins) and scheduled in § "Phase 5 —
//! Remaining built-in node kinds". Locked at D-F5.2 of the
//! `starter-flow-phase5-demo` job. Emits its input slot as a
//! structured event through the existing tracing seam noted in §
//! "R13 — Streaming, cancellation, observability reuse existing
//! seams"; no file/stdout sink — whatever subscriber the host
//! attached routes the event.
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** `log` is just a `NodeBehavior`
//!   impl that emits one `tracing::event!` per invocation.
//! - **R2 — One write chokepoint.** The body returns its output
//!   [`SlotMap`]; the propagator funnels through
//!   `GraphStore::write_slot`. The tracing event is observability,
//!   not a slot write.
//! - **R5 — Stateless behaviours.** The body is a unit struct; no
//!   shared state, no per-invocation state.
//! - **R10 — Reverse-DNS ids.** [`KIND_ID`] is verbatim under
//!   `starter.flow.*`.
//! - **R12 observability.** Every invocation opens a
//!   `log.invoke` tracing span recording `(node_id, run_id, level,
//!   cancel_observed)`.
//! - **R13 cancellation.** Sync work — the body checks
//!   `ctx.cancel.is_cancelled()` once before emitting; bounded
//!   sub-millisecond cancel-to-exit.

use std::sync::LazyLock;

use async_trait::async_trait;
use schemars::{schema::RootSchema, JsonSchema};
use serde::Deserialize;
use thiserror::Error;

use starter_flow_spi::node::{
    anyhow_compat, KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.log";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub static DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.log.label",
        "starter.flow.node.log.summary",
        "starter.flow.node.log.help",
    );

/// Mandatory input slot carrying the value to emit. The slot
/// value may be any [`SlotValue`] variant; the body Debug-formats it
/// into the tracing event's `value` field.
pub const VALUE_SLOT: &str = "value";

/// Optional config slot carrying the tracing level. Slot value must
/// be a [`SlotValue::String`] matching one of
/// `{"trace","debug","info","warn","error"}` (case-sensitive).
/// Defaults to `"info"` when absent.
pub const LEVEL_SLOT: &str = "level";

/// Output slot the body writes a passthrough copy of [`VALUE_SLOT`]
/// into so the slot can chain into a downstream node.
pub const EMITTED_SLOT: &str = "emitted";

/// Tracing target every `log` node emits under. Lets a subscriber
/// filter on the flow-log channel without scraping every flow span.
pub const TRACING_TARGET: &str = "starter.flow.log";

/// Publish-time configuration carried on a `log` node's
/// `settings:` field in a flow body. Per
/// [`DOCS/flow/scope/settings.md`](../../../DOCS/flow/scope/settings.md)
/// Phase S-4: the kind exposes a typed schema derived from this
/// struct via [`schemars`] so editor surfaces can validate drafts
/// and generate forms without re-implementing per-kind knowledge.
///
/// Runtime [`Log::invoke`] still reads [`LEVEL_SLOT`] from the
/// input [`SlotMap`]; once `TopologyResolver::resolve` lands
/// (see `DOCS/flow/scope/hot-reload.md` HR5) it will project the
/// settings here into that config slot. Until then this struct
/// only powers schema-fetch surfaces.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogSettings {
    /// Tracing level the emitted event uses. One of
    /// `"trace" | "debug" | "info" | "warn" | "error"`. Defaults
    /// to `"info"` when absent.
    #[serde(default)]
    pub level: Option<LogLevel>,
}

/// Tracing level a `log` node emits at. Mirrors the runtime values
/// [`parse_level`] accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// `tracing::Level::TRACE`.
    Trace,
    /// `tracing::Level::DEBUG`.
    Debug,
    /// `tracing::Level::INFO` — the default when absent.
    Info,
    /// `tracing::Level::WARN`.
    Warn,
    /// `tracing::Level::ERROR`.
    Error,
}

/// Derived JSON Schema for [`LogSettings`]. Returned by reference
/// from [`Log::config_schema`]; built once per process via
/// [`LazyLock`].
pub static LOG_SETTINGS_SCHEMA: LazyLock<RootSchema> =
    LazyLock::new(|| schemars::schema_for!(LogSettings));

/// Typed errors surfaced by [`Log::invoke`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LogError {
    /// The input did not carry a [`VALUE_SLOT`] entry.
    #[error("log input missing `{VALUE_SLOT}` slot")]
    MissingValue,

    /// The optional [`LEVEL_SLOT`] was present but not one of
    /// `{"trace","debug","info","warn","error"}`.
    #[error("log `level` must be one of trace|debug|info|warn|error; got `{0}`")]
    InvalidLevel(String),
}

impl LogError {
    fn into_node_error(self) -> NodeError {
        NodeError::Other(anyhow_compat::Error(Box::new(self)))
    }
}

/// `log` node-kind behaviour. Stateless (R5) — unit struct.
pub struct Log {
    kind: KindId,
}

impl Default for Log {
    fn default() -> Self {
        Self::new()
    }
}

impl Log {
    /// Construct a [`Log`] node body. Panics if [`KIND_ID`] is not a
    /// valid reverse-DNS identifier (compile-time invariant).
    pub fn new() -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
        }
    }
}

fn parse_level(s: &str) -> Result<tracing::Level, LogError> {
    match s {
        "trace" => Ok(tracing::Level::TRACE),
        "debug" => Ok(tracing::Level::DEBUG),
        "info" => Ok(tracing::Level::INFO),
        "warn" => Ok(tracing::Level::WARN),
        "error" => Ok(tracing::Level::ERROR),
        other => Err(LogError::InvalidLevel(other.to_owned())),
    }
}

#[async_trait]
impl NodeBehavior for Log {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    fn config_schema(&self) -> &'static RootSchema {
        &LOG_SETTINGS_SCHEMA
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let value = input
            .remove(VALUE_SLOT)
            .ok_or_else(|| LogError::MissingValue.into_node_error())?;

        let level = match input.remove(LEVEL_SLOT) {
            None => tracing::Level::INFO,
            Some(SlotValue::String(s)) => parse_level(&s).map_err(LogError::into_node_error)?,
            Some(_) => {
                return Err(LogError::InvalidLevel("<non-string>".to_owned()).into_node_error())
            }
        };

        // R12 observability span enclosing the emit.
        let span = tracing::info_span!(
            "log.invoke",
            node_id = %ctx.node,
            run_id = %ctx.run,
            level = %level,
            cancel_observed = tracing::field::Empty,
        );
        let _enter = span.enter();

        // R13 — single cancel check before the (sync) emit. The
        // emit itself is a tracing macro call; there's no await
        // point to select! against.
        if ctx.cancel.is_cancelled() {
            span.record("cancel_observed", true);
            return Err(NodeError::Cancelled);
        }
        span.record("cancel_observed", false);

        // The tracing::event! macro requires a literal level; match
        // and dispatch.
        match level {
            tracing::Level::TRACE => tracing::event!(
                target: TRACING_TARGET,
                tracing::Level::TRACE,
                node_id = %ctx.node,
                run_id = %ctx.run,
                value = ?value,
            ),
            tracing::Level::DEBUG => tracing::event!(
                target: TRACING_TARGET,
                tracing::Level::DEBUG,
                node_id = %ctx.node,
                run_id = %ctx.run,
                value = ?value,
            ),
            tracing::Level::INFO => tracing::event!(
                target: TRACING_TARGET,
                tracing::Level::INFO,
                node_id = %ctx.node,
                run_id = %ctx.run,
                value = ?value,
            ),
            tracing::Level::WARN => tracing::event!(
                target: TRACING_TARGET,
                tracing::Level::WARN,
                node_id = %ctx.node,
                run_id = %ctx.run,
                value = ?value,
            ),
            tracing::Level::ERROR => tracing::event!(
                target: TRACING_TARGET,
                tracing::Level::ERROR,
                node_id = %ctx.node,
                run_id = %ctx.run,
                value = ?value,
            ),
        }

        let mut out = SlotMap::new();
        out.insert(EMITTED_SLOT.to_owned(), value);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use starter_flow_spi::node::{NodeId, SlotMap, SlotValue};
    use starter_flow_spi::Cancel;
    use tracing::{
        field::{Field, Visit},
        Event, Subscriber,
    };
    use tracing_subscriber::{layer::Context, Layer, Registry};

    struct NoCancel;
    impl Cancel for NoCancel {
        fn is_cancelled(&self) -> bool {
            false
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(std::future::pending())
        }
    }

    struct AlreadyCancelled;
    impl Cancel for AlreadyCancelled {
        fn is_cancelled(&self) -> bool {
            true
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(std::future::ready(()))
        }
    }

    fn make_ctx<'a>(node: &'a NodeId, cancel: &'a dyn Cancel) -> NodeCtx<'a> {
        NodeCtx::new(
            starter_flow_spi::flow::RunId::new(),
            node,
            cancel,
            starter_flow_spi::skill::SkillSelection::NONE,
        )
    }

    /// Capture every event emitted under the `starter.flow.log`
    /// target. Records `(level, value-debug-string)` per event.
    #[derive(Default, Clone)]
    struct Captured {
        events: Arc<Mutex<Vec<(tracing::Level, String)>>>,
        count: Arc<AtomicUsize>,
    }

    struct CaptureLayer {
        captured: Captured,
    }

    struct ValueVisitor {
        value: Option<String>,
    }
    impl Visit for ValueVisitor {
        fn record_debug(&mut self, field: &Field, val: &dyn std::fmt::Debug) {
            if field.name() == "value" {
                self.value = Some(format!("{val:?}"));
            }
        }
    }

    impl<S: Subscriber> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let meta = event.metadata();
            if meta.target() != TRACING_TARGET {
                return;
            }
            let mut v = ValueVisitor { value: None };
            event.record(&mut v);
            self.captured.events.lock().unwrap().push((
                *meta.level(),
                v.value.unwrap_or_else(|| "<no value>".to_owned()),
            ));
            self.captured.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Install a thread-local subscriber whose Drop restores the
    /// previous default. Lets `#[tokio::test]` await freely while
    /// the capture layer is active.
    fn install_capture(captured: Captured) -> tracing::subscriber::DefaultGuard {
        use tracing_subscriber::layer::SubscriberExt;
        let sub = Registry::default().with(CaptureLayer { captured });
        tracing::subscriber::set_default(sub)
    }

    /// Happy path: a string value is emitted at INFO by default and
    /// the passthrough output slot holds the same value.
    #[tokio::test]
    async fn default_level_emits_info_event_and_passthrough() {
        let captured = Captured::default();
        let _guard = install_capture(captured.clone());
        let node_kind = Log::new();
        let node = NodeId::new("flow.test.log").unwrap();
        let cancel = NoCancel;

        let mut input = SlotMap::new();
        input.insert(
            VALUE_SLOT.to_owned(),
            SlotValue::String("hello demo".to_owned()),
        );

        let out = node_kind
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect("happy path must succeed");

        let events = captured.events.lock().unwrap();
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].0, tracing::Level::INFO);
        assert!(
            events[0].1.contains("hello demo"),
            "event value must contain the input string; got {}",
            events[0].1
        );

        match out.get(EMITTED_SLOT) {
            Some(SlotValue::String(s)) => assert_eq!(s, "hello demo"),
            other => panic!("expected passthrough String on `emitted`; got {other:?}"),
        }
    }

    /// Explicit level routes to the requested tracing level.
    #[tokio::test]
    async fn explicit_level_routes_to_requested_tracing_level() {
        let captured = Captured::default();
        let _guard = install_capture(captured.clone());
        let node_kind = Log::new();
        let node = NodeId::new("flow.test.log").unwrap();
        let cancel = NoCancel;

        let mut input = SlotMap::new();
        input.insert(
            VALUE_SLOT.to_owned(),
            SlotValue::String("warn me".to_owned()),
        );
        input.insert(LEVEL_SLOT.to_owned(), SlotValue::String("warn".to_owned()));

        node_kind
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect("happy path must succeed");

        let events = captured.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, tracing::Level::WARN);
    }

    /// Missing `value` slot returns `LogError::MissingValue`.
    #[tokio::test]
    async fn missing_value_returns_typed_error() {
        let node_kind = Log::new();
        let node = NodeId::new("flow.test.log").unwrap();
        let cancel = NoCancel;

        let err = node_kind
            .invoke(make_ctx(&node, &cancel), SlotMap::new())
            .await
            .expect_err("missing value must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other; got {err:?}");
        };
        let e = boxed
            .0
            .downcast::<LogError>()
            .expect("Other must wrap LogError");
        assert!(matches!(*e, LogError::MissingValue));
    }

    /// Invalid level returns `LogError::InvalidLevel`.
    #[tokio::test]
    async fn invalid_level_returns_typed_error() {
        let node_kind = Log::new();
        let node = NodeId::new("flow.test.log").unwrap();
        let cancel = NoCancel;

        let mut input = SlotMap::new();
        input.insert(VALUE_SLOT.to_owned(), SlotValue::String("x".to_owned()));
        input.insert(LEVEL_SLOT.to_owned(), SlotValue::String("loud".to_owned()));

        let err = node_kind
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect_err("invalid level must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other; got {err:?}");
        };
        let e = boxed
            .0
            .downcast::<LogError>()
            .expect("Other must wrap LogError");
        assert!(matches!(*e, LogError::InvalidLevel(ref s) if s == "loud"));
    }

    /// Cancel observed pre-emit surfaces `NodeError::Cancelled`
    /// without emitting any event.
    #[tokio::test]
    async fn already_cancelled_skips_emit_and_returns_cancelled() {
        let captured = Captured::default();
        let _guard = install_capture(captured.clone());
        let node_kind = Log::new();
        let node = NodeId::new("flow.test.log").unwrap();
        let cancel = AlreadyCancelled;

        let mut input = SlotMap::new();
        input.insert(VALUE_SLOT.to_owned(), SlotValue::String("x".to_owned()));

        let err = node_kind
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect_err("cancelled invocation must return Err");
        assert!(matches!(err, NodeError::Cancelled));
        assert_eq!(captured.count.load(Ordering::SeqCst), 0);
    }
}
