//! `tool-call` — wraps any registered `starter_spi::Tool`.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R8 — Nodes are
//! not Tools; Tools are one node kind": the input slot accepts the
//! tool's input shape, the output slot carries its return value, and
//! every registered `Tool` is invocable from any flow via this single
//! node kind. Also listed in § "Phase 2 — `starter-flow` engine
//! (in-memory stores)" as one of the two built-ins shipped with the
//! engine.
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** `tool-call` is the canonical
//!   bridge between the flow graph and the tool registry.
//! - **R2 — One write chokepoint.** The body returns its output
//!   [`SlotMap`]; the propagator funnels every value through the
//!   single [`GraphStore::write_slot`] call. We never bypass it.
//! - **R5 — Stateless behaviours.** `&self`, never `&mut self`. The
//!   registry handle is a `Arc<dyn ToolRegistry>` — shared,
//!   immutable, host-built.
//! - **R10 — Reverse-DNS ids; namespace ownership.** [`KIND_ID`]
//!   stays verbatim under `starter.flow.*`. Tool ids are validated
//!   as [`KindId`] reverse-DNS strings — the same id starter-mcp
//!   lists.
//! - **R12 observability.** Every invocation opens a
//!   `tool_call.invoke` tracing span recording `(node_id, tool_id,
//!   principal_id_hash, cancel_observed)`.
//! - **R13 cancellation.** [`NodeCtx::cancel`] races the tool's
//!   future inside a `tokio::select!`. A fired cancel drops the
//!   tool's in-flight call and surfaces as
//!   [`NodeError::Cancelled`] in bounded time.
//! - **Extensions R13 — adapters apply auth, not nodes.** This
//!   crate trusts the `Principal` it is handed; the surface adapter
//!   (REST / MCP / CLI / JSON-RPC) is responsible for the authz
//!   check before the engine sees the call.
//!
//! [`GraphStore::write_slot`]: starter_flow_spi::graph::GraphStore::write_slot

use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

use starter_flow_spi::node::{
    anyhow_compat, KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.tool-call";

/// Input slot carrying the [`KindId`] of the tool to invoke. The slot
/// value must be a [`SlotValue::String`] holding a reverse-DNS id; the
/// body validates it through [`KindId::new`] and rejects malformed ids
/// with [`ToolCallError::InvalidToolId`].
pub const TOOL_ID_SLOT: &str = "tool_id";

/// Input slot carrying the tool's input JSON payload. The slot value
/// must be a [`SlotValue::Json`]; the body forwards the inner
/// [`serde_json::Value`] to [`Tool::invoke`] verbatim (the surface
/// adapter has already validated it against the tool's
/// `input_schema`).
pub const TOOL_INPUT_SLOT: &str = "input";

/// Output slot the body writes the tool's [`serde_json::Value`]
/// return into, wrapped as [`SlotValue::Json`]. The propagator funnels
/// it through the single `GraphStore::write_slot` chokepoint.
pub const TOOL_OUTPUT_SLOT: &str = "output";

/// Host-controlled tool registry the [`ToolCall`] body resolves
/// `tool_id` against.
///
/// Re-exported from [`crate::tool_registry`] so the trait lives in
/// an always-compiled module shared with the `ai-agent` body
/// (D-F4.9 single chokepoint), without forcing the `ai-agent`
/// feature to enable `tool-call`.
pub use crate::tool_registry::{StaticToolRegistry, ToolRegistry};

/// Typed errors surfaced by [`ToolCall::invoke`].
///
/// `Cancelled` is mapped to [`NodeError::Cancelled`] (R13). Every
/// other variant becomes a [`NodeError::Other`] over the
/// [`NodeBehavior`] seam; the propagator turns that into
/// [`starter_flow_spi::flow::FlowEvent::NodeFailed`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ToolCallError {
    /// The input did not carry a string-valued [`TOOL_ID_SLOT`].
    #[error(
        "tool-call input missing `{TOOL_ID_SLOT}` slot \
         (must be SlotValue::String naming the tool's KindId)"
    )]
    MissingToolId,

    /// The `tool_id` was a string but not a valid reverse-DNS
    /// [`KindId`].
    #[error("tool-call `tool_id` is not a valid reverse-DNS KindId: {0}")]
    InvalidToolId(String),

    /// The input did not carry a JSON-valued [`TOOL_INPUT_SLOT`].
    #[error(
        "tool-call input missing `{TOOL_INPUT_SLOT}` slot \
         (must be SlotValue::Json carrying the tool's input payload)"
    )]
    MissingInput,

    /// The `tool_id` parsed but no tool is registered under it.
    #[error("tool-call tool not registered: {0}")]
    UnregisteredTool(KindId),

    /// The tool itself returned a typed error.
    #[error("tool `{tool_id}` failed: {message}")]
    ToolFailed {
        /// The id of the tool that failed.
        tool_id: KindId,
        /// Stringified `starter_spi::error::Error` from the tool.
        message: String,
    },
}

impl ToolCallError {
    fn into_node_error(self) -> NodeError {
        NodeError::Other(anyhow_compat::Error(Box::new(self)))
    }
}

/// `tool-call` node-kind behaviour.
///
/// Stateless (R5) — the only state on the struct is the precomputed
/// [`KindId`] and the host-supplied `Arc<dyn ToolRegistry>`. Both are
/// shared, immutable, and safe to call concurrently.
pub struct ToolCall {
    kind: KindId,
    registry: Arc<dyn ToolRegistry>,
}

impl ToolCall {
    /// Construct a [`ToolCall`] backed by the given tool registry.
    ///
    /// Panics if [`KIND_ID`] is not a valid reverse-DNS id — which is
    /// a compile-time-checkable invariant of this crate (the constant
    /// is verbatim per stage 1's lock).
    pub fn new(registry: Arc<dyn ToolRegistry>) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
            registry,
        }
    }
}

#[async_trait]
impl NodeBehavior for ToolCall {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        // Pull `tool_id` (validated as a KindId per R10 reverse-DNS).
        let tool_id_raw = match input.remove(TOOL_ID_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => return Err(ToolCallError::MissingToolId.into_node_error()),
        };
        let tool_id = KindId::new(tool_id_raw.clone())
            .map_err(|_| ToolCallError::InvalidToolId(tool_id_raw).into_node_error())?;

        // Pull `input` JSON payload.
        let payload = match input.remove(TOOL_INPUT_SLOT) {
            Some(SlotValue::Json(v)) => v,
            _ => return Err(ToolCallError::MissingInput.into_node_error()),
        };

        // R12 observability span. `principal_id_hash` is left
        // `Empty`: NodeCtx does not yet thread `Principal` (the SPI
        // marks the struct `#[non_exhaustive]` to add it later); the
        // adapter on the boundary records the principal in its own
        // span which encloses this one. `cancel_observed` is recorded
        // post-invocation via `Span::record`.
        let span = tracing::info_span!(
            "tool_call.invoke",
            node_id = %ctx.node,
            tool_id = %tool_id,
            principal_id_hash = tracing::field::Empty,
            cancel_observed = tracing::field::Empty,
        );
        let _enter = span.enter();

        let tool = match self.registry.lookup(&tool_id) {
            Some(t) => t,
            None => {
                tracing::warn!(tool_id = %tool_id, "tool_id not registered");
                span.record("cancel_observed", false);
                return Err(ToolCallError::UnregisteredTool(tool_id).into_node_error());
            }
        };

        // R13 cancellation. The `Tool::invoke` signature does not
        // accept a `Cancel` directly, so we race its future against
        // `ctx.cancel.cancelled()` inside a `select!`. A fired cancel
        // drops the in-flight future on the next yield point.
        let cancelled = ctx.cancel.cancelled();
        let invocation = tool.invoke(payload);
        tokio::pin!(invocation);
        tokio::pin!(cancelled);

        let result = tokio::select! {
            biased;
            _ = &mut cancelled => {
                span.record("cancel_observed", true);
                tracing::info!(tool_id = %tool_id, "tool_call cancelled mid-invocation");
                return Err(NodeError::Cancelled);
            }
            r = &mut invocation => r,
        };
        span.record("cancel_observed", false);

        match result {
            Ok(value) => {
                let mut out = SlotMap::new();
                out.insert(TOOL_OUTPUT_SLOT.to_owned(), SlotValue::Json(value));
                Ok(out)
            }
            Err(e) => {
                let message = e.to_string();
                tracing::warn!(tool_id = %tool_id, error = %message, "tool returned error");
                Err(ToolCallError::ToolFailed { tool_id, message }.into_node_error())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;
    use std::time::Duration;

    use starter_flow_spi::node::{NodeId, SlotMap, SlotValue};
    use starter_flow_spi::Cancel;
    use starter_spi::error::{Error as SpiError, Result as SpiResult};
    use starter_spi::tool::{Tool, ToolDefinition};

    /// Always-live cancel — the unit tests that don't exercise R13
    /// never fire it.
    struct NoCancel;
    impl Cancel for NoCancel {
        fn is_cancelled(&self) -> bool {
            false
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(std::future::pending())
        }
    }

    /// Manually-fireable cancel for the cancellation test.
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

    /// Mock tool that records every invocation's input and replays a
    /// canned response. Constructed per-test; both the input log and
    /// the response are `Send + Sync` so the tool can live behind an
    /// `Arc<dyn Tool>`.
    struct MockTool {
        id: String,
        calls: Mutex<Vec<serde_json::Value>>,
        response: MockResponse,
    }

    enum MockResponse {
        /// Return this value verbatim.
        Ok(serde_json::Value),
        /// Return an `SpiError::Invalid` carrying a canned message.
        /// The variant is unit-typed because `SpiError` is `!Clone`
        /// and the mock rebuilds a fresh error on each invocation.
        Err,
        /// Sleep this long before returning `Ok(Null)`. Used by the
        /// cancellation test — the cancel must abort the sleep.
        Sleep(Duration),
    }

    impl MockTool {
        fn ok(id: &str, response: serde_json::Value) -> Self {
            Self {
                id: id.to_owned(),
                calls: Mutex::new(Vec::new()),
                response: MockResponse::Ok(response),
            }
        }
        fn err(id: &str) -> Self {
            Self {
                id: id.to_owned(),
                calls: Mutex::new(Vec::new()),
                response: MockResponse::Err,
            }
        }
        fn sleeping(id: &str, dur: Duration) -> Self {
            Self {
                id: id.to_owned(),
                calls: Mutex::new(Vec::new()),
                response: MockResponse::Sleep(dur),
            }
        }
        fn calls(&self) -> Vec<serde_json::Value> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: self.id.clone(),
                description: "mock".to_owned(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }

        async fn invoke(&self, input: serde_json::Value) -> SpiResult<serde_json::Value> {
            self.calls.lock().unwrap().push(input);
            match &self.response {
                MockResponse::Ok(v) => Ok(v.clone()),
                MockResponse::Err => Err(SpiError::Invalid {
                    message: "mock tool deliberate failure".to_owned(),
                }),
                MockResponse::Sleep(d) => {
                    tokio::time::sleep(*d).await;
                    Ok(serde_json::Value::Null)
                }
            }
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

    fn tool_id(s: &str) -> KindId {
        KindId::new(s).unwrap()
    }

    fn input_with(tool_id: &str, payload: serde_json::Value) -> SlotMap {
        let mut m = SlotMap::new();
        m.insert(
            TOOL_ID_SLOT.to_owned(),
            SlotValue::String(tool_id.to_owned()),
        );
        m.insert(TOOL_INPUT_SLOT.to_owned(), SlotValue::Json(payload));
        m
    }

    /// Happy path: the tool's output JSON is propagated to the
    /// `output` slot, and the tool sees the input verbatim.
    #[tokio::test]
    async fn successful_invocation_propagates_output_json() {
        let mock = Arc::new(MockTool::ok(
            "com.acme.echo",
            serde_json::json!({"echoed": true}),
        ));
        let mut registry = StaticToolRegistry::new();
        registry.register(tool_id("com.acme.echo"), mock.clone());
        let node_kind = ToolCall::new(Arc::new(registry));

        let node = NodeId::new("flow.test.tc").unwrap();
        let cancel = NoCancel;
        let payload = serde_json::json!({"q": 42});
        let out = node_kind
            .invoke(
                make_ctx(&node, &cancel),
                input_with("com.acme.echo", payload.clone()),
            )
            .await
            .expect("happy path must succeed");

        assert_eq!(out.len(), 1);
        match out.get(TOOL_OUTPUT_SLOT) {
            Some(SlotValue::Json(v)) => assert_eq!(v, &serde_json::json!({"echoed": true})),
            other => panic!("expected SlotValue::Json on `output`; got {other:?}"),
        }
        // MockTool recorded the input verbatim — proves the body
        // forwards rather than mangling.
        assert_eq!(mock.calls(), vec![payload]);
    }

    /// Tool returning a typed error surfaces as
    /// `ToolCallError::ToolFailed` over the NodeBehavior seam (the
    /// propagator turns this into `FlowEvent::NodeFailed`).
    #[tokio::test]
    async fn tool_error_surfaces_as_node_error() {
        let mock = Arc::new(MockTool::err("com.acme.bad"));
        let mut registry = StaticToolRegistry::new();
        registry.register(tool_id("com.acme.bad"), mock);
        let node_kind = ToolCall::new(Arc::new(registry));

        let node = NodeId::new("flow.test.tc").unwrap();
        let cancel = NoCancel;
        let err = node_kind
            .invoke(
                make_ctx(&node, &cancel),
                input_with("com.acme.bad", serde_json::json!({})),
            )
            .await
            .expect_err("typed tool error must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other(ToolCallError::ToolFailed); got {err:?}");
        };
        let tc = boxed
            .0
            .downcast::<ToolCallError>()
            .expect("Other must wrap ToolCallError");
        match *tc {
            ToolCallError::ToolFailed {
                ref tool_id,
                ref message,
            } => {
                assert_eq!(tool_id.as_str(), "com.acme.bad");
                assert!(
                    message.contains("invalid input"),
                    "ToolFailed.message must include the tool error display; got {message:?}",
                );
            }
            ref other => panic!("expected ToolFailed; got {other:?}"),
        }
    }

    /// A `Cancel` firing mid-invocation aborts the tool's in-flight
    /// future in bounded time (<200ms) and the body returns
    /// `NodeError::Cancelled`.
    #[tokio::test]
    async fn cancel_mid_invocation_aborts_within_bounded_time() {
        let mock = Arc::new(MockTool::sleeping("com.acme.slow", Duration::from_secs(30)));
        let mut registry = StaticToolRegistry::new();
        registry.register(tool_id("com.acme.slow"), mock);
        let node_kind = Arc::new(ToolCall::new(Arc::new(registry)));

        let cancel = Arc::new(FlagCancel::new());
        let cancel_for_fire = cancel.clone();

        let invoke_handle = tokio::spawn({
            let node_kind = node_kind.clone();
            let cancel = cancel.clone();
            async move {
                let node = NodeId::new("flow.test.tc").unwrap();
                let ctx = NodeCtx::new(
                    starter_flow_spi::flow::RunId::new(),
                    // Use a leaked id to satisfy the `'a` borrow
                    // inside the spawned task without juggling
                    // owned/borrowed split.
                    Box::leak(Box::new(node)),
                    cancel.as_ref(),
                    starter_flow_spi::skill::SkillSelection::NONE,
                );
                node_kind
                    .invoke(ctx, input_with("com.acme.slow", serde_json::json!({})))
                    .await
            }
        });

        // Let the invocation enter the sleep, then fire cancel.
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel_for_fire.fire();

        let result = tokio::time::timeout(Duration::from_millis(200), invoke_handle)
            .await
            .expect("cancel must abort the in-flight tool call within 200ms")
            .expect("invoke task must not panic");
        let err = result.expect_err("cancelled invocation must return Err");
        assert!(
            matches!(err, NodeError::Cancelled),
            "cancelled invocation must surface as NodeError::Cancelled; got {err:?}",
        );
    }

    /// A `tool_id` that does not resolve in the registry surfaces as
    /// `ToolCallError::UnregisteredTool`.
    #[tokio::test]
    async fn unregistered_tool_id_returns_typed_error() {
        let registry: Arc<dyn ToolRegistry> = Arc::new(StaticToolRegistry::new());
        let node_kind = ToolCall::new(registry);

        let node = NodeId::new("flow.test.tc").unwrap();
        let cancel = NoCancel;
        let err = node_kind
            .invoke(
                make_ctx(&node, &cancel),
                input_with("com.acme.missing", serde_json::json!({})),
            )
            .await
            .expect_err("unregistered tool must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other(ToolCallError::UnregisteredTool); got {err:?}");
        };
        let tc = boxed
            .0
            .downcast::<ToolCallError>()
            .expect("Other must wrap ToolCallError");
        assert!(
            matches!(*tc, ToolCallError::UnregisteredTool(ref id) if id.as_str() == "com.acme.missing"),
            "expected UnregisteredTool(com.acme.missing); got {tc:?}",
        );
    }

    /// Missing `tool_id` slot returns the typed
    /// `ToolCallError::MissingToolId`.
    #[tokio::test]
    async fn missing_tool_id_returns_typed_error() {
        let registry: Arc<dyn ToolRegistry> = Arc::new(StaticToolRegistry::new());
        let node_kind = ToolCall::new(registry);

        let node = NodeId::new("flow.test.tc").unwrap();
        let cancel = NoCancel;
        let mut input = SlotMap::new();
        input.insert(
            TOOL_INPUT_SLOT.to_owned(),
            SlotValue::Json(serde_json::json!({})),
        );

        let err = node_kind
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect_err("missing tool_id must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other(ToolCallError::MissingToolId); got {err:?}");
        };
        let tc = boxed
            .0
            .downcast::<ToolCallError>()
            .expect("Other must wrap ToolCallError");
        assert!(
            matches!(*tc, ToolCallError::MissingToolId),
            "expected MissingToolId; got {tc:?}",
        );
    }

    /// A malformed (non-reverse-DNS) `tool_id` returns
    /// `ToolCallError::InvalidToolId`.
    #[tokio::test]
    async fn invalid_tool_id_returns_typed_error() {
        let registry: Arc<dyn ToolRegistry> = Arc::new(StaticToolRegistry::new());
        let node_kind = ToolCall::new(registry);

        let node = NodeId::new("flow.test.tc").unwrap();
        let cancel = NoCancel;
        let err = node_kind
            .invoke(
                make_ctx(&node, &cancel),
                input_with("NotReverseDns", serde_json::json!({})),
            )
            .await
            .expect_err("invalid tool_id must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other(ToolCallError::InvalidToolId); got {err:?}");
        };
        let tc = boxed
            .0
            .downcast::<ToolCallError>()
            .expect("Other must wrap ToolCallError");
        assert!(
            matches!(*tc, ToolCallError::InvalidToolId(ref s) if s == "NotReverseDns"),
            "expected InvalidToolId(\"NotReverseDns\"); got {tc:?}",
        );
    }
}
