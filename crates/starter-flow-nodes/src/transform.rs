//! `transform` — pure-function node kind.
//!
//! Semantics: a stateless map over slot values. Phase 2 stage 1 of the
//! `starter-flow-engine-finish` job locked the substrate as a registered
//! Rust closure indexed by a `fn_id` slot value (no `rhai` dep on
//! `starter-flow-nodes`); the host constructs a
//! [`TransformFunctionRegistry`] at engine-build time, threads an
//! [`Arc`] into the [`Transform`] behaviour, and registers the kind via
//! the host-only `NodeKindRegistry::register_builtin` path under the
//! reserved `starter.flow.*` namespace (SCOPE R10).
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** `transform` is one of the
//!   canonical examples in § "R1 — Everything is a Node".
//! - **R2 — One write chokepoint.** This body returns its output
//!   [`SlotMap`]; the propagator funnels every value through the
//!   single [`GraphStore::write_slot`] call. We never bypass it.
//! - **R5 — Stateless behaviours.** `&self`, never `&mut self`. The
//!   registry handle is a `Arc<dyn TransformFunctionRegistry>` —
//!   shared, immutable, host-built.
//! - **R10 — Reverse-DNS ids; namespace ownership.** [`KIND_ID`]
//!   stays verbatim under `starter.flow.*`.
//! - **R12 observability.** Every invocation opens a
//!   `transform.invoke` tracing span recording `(node_id, fn_id,
//!   input_kind, output_kind)`.
//!
//! [`GraphStore::write_slot`]: starter_flow_spi::graph::GraphStore::write_slot

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

use starter_flow_spi::node::{
    anyhow_compat, KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.transform";

/// Input slot name carrying the `fn_id` string — the key the registry
/// is looked up by.
///
/// The flow definition wires this slot's value at engine-build time
/// (or pre-seeds it before the run); the propagator passes the full
/// input [`SlotMap`] into [`Transform::invoke`] and this body splits
/// the `fn_id` selector from the actual transform input.
pub const FN_ID_SLOT: &str = "fn_id";

/// A registered transform closure.
///
/// Sync, `Send + Sync + 'static` because the same closure may be
/// invoked concurrently across runs and is shared by [`Arc`]. The
/// input is the [`SlotMap`] the propagator read off the node's input
/// slots (with [`FN_ID_SLOT`] already filtered out by [`Transform`]);
/// the output [`SlotMap`] is written back through the single
/// [`GraphStore::write_slot`] chokepoint by the propagator.
///
/// Returning `SlotMap` (rather than `Result`) keeps the seam minimal —
/// closures signal "this input is bad" by panicking and the body
/// catches the unwind into a typed [`TransformError::Panicked`]. The
/// propagator turns any `Err` from [`NodeBehavior::invoke`] into
/// [`starter_flow_spi::flow::FlowEvent::NodeFailed`].
///
/// [`GraphStore::write_slot`]: starter_flow_spi::graph::GraphStore::write_slot
pub type TransformFn = Arc<dyn Fn(SlotMap) -> SlotMap + Send + Sync + 'static>;

/// Host-controlled function registry the [`Transform`] body resolves
/// `fn_id` against.
///
/// SCOPE R5 keeps the body stateless: the registry is an `Arc<dyn>`
/// handed in at construction time and the trait surface is read-only.
/// The host constructs and freezes the registry at engine-build time;
/// the body never mutates it.
pub trait TransformFunctionRegistry: Send + Sync + 'static {
    /// Look up a transform function by id. Returns `None` if no
    /// function is registered under the given id.
    fn lookup(&self, fn_id: &str) -> Option<TransformFn>;
}

/// In-memory [`TransformFunctionRegistry`] populated at engine-build
/// time.
///
/// Mutation is confined to the builder phase ([`Self::register`]); once
/// the registry is wrapped in an [`Arc<dyn TransformFunctionRegistry>`]
/// and handed to [`Transform::new`], it is read-only.
#[derive(Default)]
pub struct StaticTransformFunctionRegistry {
    fns: HashMap<String, TransformFn>,
}

impl StaticTransformFunctionRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a closure under an id. Replaces any previous entry
    /// under the same id (the registry is host-owned, so collisions
    /// are a host-build bug, not a runtime error).
    pub fn register<F>(&mut self, fn_id: impl Into<String>, f: F)
    where
        F: Fn(SlotMap) -> SlotMap + Send + Sync + 'static,
    {
        self.fns.insert(fn_id.into(), Arc::new(f));
    }

    /// Register a pre-built [`TransformFn`] — convenience for callers
    /// that already hold an `Arc`.
    pub fn register_arc(&mut self, fn_id: impl Into<String>, f: TransformFn) {
        self.fns.insert(fn_id.into(), f);
    }
}

impl TransformFunctionRegistry for StaticTransformFunctionRegistry {
    fn lookup(&self, fn_id: &str) -> Option<TransformFn> {
        self.fns.get(fn_id).cloned()
    }
}

/// Typed errors surfaced by [`Transform::invoke`].
///
/// Each variant becomes a [`NodeError::Other`] over the
/// [`NodeBehavior`] seam; the propagator turns that into
/// [`starter_flow_spi::flow::FlowEvent::NodeFailed`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TransformError {
    /// The input did not carry a string-valued [`FN_ID_SLOT`].
    #[error(
        "transform input missing `{FN_ID_SLOT}` slot \
         (must be SlotValue::String naming a registered function)"
    )]
    MissingFnId,

    /// The `fn_id` was a string but no closure is registered under it.
    #[error("transform function not registered: {0}")]
    UnregisteredFn(String),

    /// The registered closure panicked. The unwind is caught so the
    /// engine surfaces the failure as `FlowEvent::NodeFailed` rather
    /// than crashing the propagator task.
    #[error("transform function `{fn_id}` panicked: {message}")]
    Panicked {
        /// The id of the closure that panicked.
        fn_id: String,
        /// Best-effort stringification of the panic payload.
        message: String,
    },
}

impl TransformError {
    fn into_node_error(self) -> NodeError {
        NodeError::Other(anyhow_compat::Error(Box::new(self)))
    }
}

/// `transform` node-kind behaviour.
///
/// Stateless (R5) — the only state on the struct is the precomputed
/// [`KindId`] and the host-supplied [`Arc<dyn TransformFunctionRegistry>`].
/// Both are shared, immutable, and safe to call concurrently.
pub struct Transform {
    kind: KindId,
    registry: Arc<dyn TransformFunctionRegistry>,
}

impl Transform {
    /// Construct a [`Transform`] backed by the given function
    /// registry.
    ///
    /// Panics if [`KIND_ID`] is not a valid reverse-DNS id — which is
    /// a compile-time-checkable invariant of this crate (the constant
    /// is verbatim per stage 1's lock).
    pub fn new(registry: Arc<dyn TransformFunctionRegistry>) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
            registry,
        }
    }
}

#[async_trait]
impl NodeBehavior for Transform {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        // Pull `fn_id` out of the input map so the registered closure
        // sees only the actual payload slots. The flow definition
        // wires `fn_id` from a config-style upstream (a constant or a
        // selector slot); the body itself is config-agnostic.
        let fn_id = match input.remove(FN_ID_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => return Err(TransformError::MissingFnId.into_node_error()),
        };

        // R12 observability span. `input_kind` is recorded up front
        // (we know it before invoking); `output_kind` is recorded
        // after the closure returns, via `Span::record`.
        let span = tracing::info_span!(
            "transform.invoke",
            node_id = %ctx.node,
            fn_id = %fn_id,
            input_kind = %slot_map_kind(&input),
            output_kind = tracing::field::Empty,
        );
        let _enter = span.enter();

        let func = match self.registry.lookup(&fn_id) {
            Some(f) => f,
            None => {
                tracing::warn!(fn_id = %fn_id, "transform fn_id not registered");
                return Err(TransformError::UnregisteredFn(fn_id).into_node_error());
            }
        };

        // Catch panics so a bad closure surfaces as
        // `FlowEvent::NodeFailed` rather than crashing the propagator
        // task. The closure is synchronous (TransformFn is `Fn`, not
        // `async fn`); `catch_unwind` + `AssertUnwindSafe` is the
        // right tool. We do not poison shared state — `func` is an
        // `Arc<dyn Fn>` and the only owned input is the `SlotMap` we
        // hand to it.
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| func(input)));
        match result {
            Ok(output) => {
                span.record(
                    "output_kind",
                    tracing::field::display(slot_map_kind(&output)),
                );
                Ok(output)
            }
            Err(payload) => {
                let message = panic_payload_to_string(payload);
                tracing::error!(
                    fn_id = %fn_id,
                    panic = %message,
                    "transform closure panicked; surfacing as NodeFailed",
                );
                Err(TransformError::Panicked { fn_id, message }.into_node_error())
            }
        }
    }
}

/// Render a [`SlotMap`]'s shape for the `input_kind` / `output_kind`
/// span fields. Format: `{slot_a=int, slot_b=string}` — the variant
/// name of each [`SlotValue`], in slot-name order (BTreeMap iteration
/// is sorted, so this is deterministic).
fn slot_map_kind(map: &SlotMap) -> String {
    let mut s = String::from("{");
    let mut first = true;
    for (k, v) in map {
        if !first {
            s.push_str(", ");
        }
        first = false;
        s.push_str(k);
        s.push('=');
        s.push_str(slot_value_kind(v));
    }
    s.push('}');
    s
}

fn slot_value_kind(v: &SlotValue) -> &'static str {
    match v {
        SlotValue::Null => "null",
        SlotValue::Bool(_) => "bool",
        SlotValue::Int(_) => "int",
        SlotValue::Float(_) => "float",
        SlotValue::String(_) => "string",
        SlotValue::Bytes(_) => "bytes",
        SlotValue::Json(_) => "json",
        _ => "unknown",
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        return (*s).to_owned();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "<non-string panic payload>".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    use starter_flow_spi::node::{NodeId, SlotMap, SlotValue};
    use starter_flow_spi::Cancel;
    use std::future::Future;
    use std::pin::Pin;

    /// Always-live cancel — the unit tests never fire it.
    struct NoCancel;
    impl Cancel for NoCancel {
        fn is_cancelled(&self) -> bool {
            false
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(std::future::pending())
        }
    }

    fn fixture_registry() -> Arc<StaticTransformFunctionRegistry> {
        let mut r = StaticTransformFunctionRegistry::new();
        // Identity: copy `value` to `value`.
        r.register("identity", |input: SlotMap| -> SlotMap { input });
        // Sum: read `a` and `b` ints, emit `sum`.
        r.register("sum", |input: SlotMap| -> SlotMap {
            let a = match input.get("a") {
                Some(SlotValue::Int(n)) => *n,
                _ => 0,
            };
            let b = match input.get("b") {
                Some(SlotValue::Int(n)) => *n,
                _ => 0,
            };
            let mut out = SlotMap::new();
            out.insert("sum".to_owned(), SlotValue::Int(a + b));
            out
        });
        // Panic: blow up.
        r.register("boom", |_input: SlotMap| -> SlotMap {
            panic!("intentional test panic")
        });
        Arc::new(r)
    }

    fn make_ctx<'a>(node: &'a NodeId, cancel: &'a NoCancel) -> NodeCtx<'a> {
        NodeCtx::new(starter_flow_spi::flow::RunId::new(), node, cancel)
    }

    /// Identity transform emits its input verbatim on output.
    #[tokio::test]
    async fn identity_transform_emits_input_verbatim() {
        let registry = fixture_registry();
        let transform = Transform::new(registry);
        let node = NodeId::new("flow.test.identity").unwrap();
        let cancel = NoCancel;

        let mut input = SlotMap::new();
        input.insert(
            FN_ID_SLOT.to_owned(),
            SlotValue::String("identity".to_owned()),
        );
        input.insert("payload".to_owned(), SlotValue::String("hello".to_owned()));
        input.insert("count".to_owned(), SlotValue::Int(7));

        let out = transform
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect("identity must succeed");

        // `fn_id` is stripped before the closure sees it; the rest is
        // emitted verbatim.
        assert_eq!(out.len(), 2);
        assert_eq!(
            out.get("payload"),
            Some(&SlotValue::String("hello".to_owned()))
        );
        assert_eq!(out.get("count"), Some(&SlotValue::Int(7)));
    }

    /// Arithmetic transform sums two input slots into one output.
    #[tokio::test]
    async fn arithmetic_transform_sums_two_slots() {
        let registry = fixture_registry();
        let transform = Transform::new(registry);
        let node = NodeId::new("flow.test.adder").unwrap();
        let cancel = NoCancel;

        let mut input = SlotMap::new();
        input.insert(FN_ID_SLOT.to_owned(), SlotValue::String("sum".to_owned()));
        input.insert("a".to_owned(), SlotValue::Int(3));
        input.insert("b".to_owned(), SlotValue::Int(4));

        let out = transform
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect("sum must succeed");

        assert_eq!(out.len(), 1);
        assert_eq!(out.get("sum"), Some(&SlotValue::Int(7)));
    }

    /// A panicking closure surfaces as a typed [`TransformError::Panicked`]
    /// over the [`NodeBehavior`] seam — *not* as a propagator-task
    /// crash. The end-to-end `FlowEvent::NodeFailed` surface is
    /// covered by the integration test in
    /// `tests/transform_node_failed.rs`.
    #[tokio::test]
    async fn panicking_transform_surfaces_as_node_error_not_crash() {
        let registry = fixture_registry();
        let transform = Transform::new(registry);
        let node = NodeId::new("flow.test.boom").unwrap();
        let cancel = NoCancel;

        let mut input = SlotMap::new();
        input.insert(FN_ID_SLOT.to_owned(), SlotValue::String("boom".to_owned()));

        let err = transform
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect_err("panic must surface as Err");

        // The error must be Other-wrapped TransformError::Panicked.
        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other(TransformError::Panicked); got {err:?}");
        };
        let tx = boxed
            .0
            .downcast::<TransformError>()
            .expect("Other must wrap TransformError");
        match *tx {
            TransformError::Panicked {
                ref fn_id,
                ref message,
            } => {
                assert_eq!(fn_id, "boom");
                assert!(
                    message.contains("intentional test panic"),
                    "panic message must round-trip the payload; got {message:?}",
                );
            }
            ref other => panic!("expected Panicked; got {other:?}"),
        }
    }

    /// An unregistered `fn_id` returns a typed
    /// [`TransformError::UnregisteredFn`].
    #[tokio::test]
    async fn unregistered_fn_id_returns_typed_error() {
        let registry = fixture_registry();
        let transform = Transform::new(registry);
        let node = NodeId::new("flow.test.unknown").unwrap();
        let cancel = NoCancel;

        let mut input = SlotMap::new();
        input.insert(
            FN_ID_SLOT.to_owned(),
            SlotValue::String("does-not-exist".to_owned()),
        );

        let err = transform
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect_err("unregistered must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other(TransformError::UnregisteredFn); got {err:?}");
        };
        let tx = boxed
            .0
            .downcast::<TransformError>()
            .expect("Other must wrap TransformError");
        assert!(
            matches!(*tx, TransformError::UnregisteredFn(ref s) if s == "does-not-exist"),
            "expected UnregisteredFn(\"does-not-exist\"); got {tx:?}",
        );
    }

    /// Missing-`fn_id` input returns the typed [`TransformError::MissingFnId`].
    #[tokio::test]
    async fn missing_fn_id_returns_typed_error() {
        let registry = fixture_registry();
        let transform = Transform::new(registry);
        let node = NodeId::new("flow.test.no-fnid").unwrap();
        let cancel = NoCancel;

        // No `fn_id` slot.
        let input = SlotMap::new();

        let err = transform
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect_err("missing fn_id must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other(TransformError::MissingFnId); got {err:?}");
        };
        let tx = boxed
            .0
            .downcast::<TransformError>()
            .expect("Other must wrap TransformError");
        assert!(
            matches!(*tx, TransformError::MissingFnId),
            "expected MissingFnId; got {tx:?}",
        );
    }
}
