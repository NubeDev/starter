//! `branch` — conditional routing node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R1 — Everything
//! is a Node" (a branch / merge / loop-condition is a node) and
//! scheduled in § "Phase 5 — Remaining built-in node kinds". The
//! `branch` evaluates the [`CONDITION_SLOT`] over its input and
//! forwards [`VALUE_SLOT`] to one of [`TRUE_OUT_SLOT`] /
//! [`FALSE_OUT_SLOT`] — never both. Downstream edges pick which
//! output they listen on, which is how the engine expresses
//! "take this branch".
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** `branch` is a plain
//!   [`NodeBehavior`] impl.
//! - **R2 — One write chokepoint.** The body returns the output
//!   [`SlotMap`]; the propagator routes through
//!   `GraphStore::write_slot`. Exactly one of the two output slots is
//!   populated per invocation.
//! - **R5 — Stateless behaviours.** Unit struct.
//! - **R10 — Reverse-DNS ids.** [`KIND_ID`] is verbatim under
//!   `starter.flow.*`.
//! - **R12 observability.** Every invocation opens a `branch.invoke`
//!   tracing span recording `(node_id, run_id, taken, cancel_observed)`.
//! - **R13 cancellation.** Sync work — a single
//!   `ctx.cancel.is_cancelled()` check before emitting.

use async_trait::async_trait;
use thiserror::Error;

use starter_flow_spi::node::{
    anyhow_compat, KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.branch";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub const DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.branch.label",
        "starter.flow.node.branch.summary",
        "starter.flow.node.branch.help",
    );

/// Mandatory input slot carrying the predicate. Truthy semantics:
/// `Bool(true)`, non-zero `Int`/`Float`, non-empty `String` / `Bytes`,
/// non-empty / non-null `Json` are truthy. `Null` and the other
/// "empty" forms are falsy.
pub const CONDITION_SLOT: &str = "condition";

/// Mandatory input slot carrying the payload forwarded to whichever
/// branch is taken.
pub const VALUE_SLOT: &str = "value";

/// Output slot populated when [`CONDITION_SLOT`] is truthy.
pub const TRUE_OUT_SLOT: &str = "true_out";

/// Output slot populated when [`CONDITION_SLOT`] is falsy.
pub const FALSE_OUT_SLOT: &str = "false_out";

/// Typed errors surfaced by [`Branch::invoke`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BranchError {
    /// The input did not carry a [`CONDITION_SLOT`] entry.
    #[error("branch input missing `{CONDITION_SLOT}` slot")]
    MissingCondition,

    /// The input did not carry a [`VALUE_SLOT`] entry.
    #[error("branch input missing `{VALUE_SLOT}` slot")]
    MissingValue,
}

impl BranchError {
    fn into_node_error(self) -> NodeError {
        NodeError::Other(anyhow_compat::Error(Box::new(self)))
    }
}

/// `branch` node-kind behaviour. Stateless (R5) — unit struct.
pub struct Branch {
    kind: KindId,
}

impl Default for Branch {
    fn default() -> Self {
        Self::new()
    }
}

impl Branch {
    /// Construct a [`Branch`] node body. Panics if [`KIND_ID`] is not
    /// a valid reverse-DNS identifier (compile-time invariant).
    pub fn new() -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
        }
    }
}

/// Truthiness rules for [`SlotValue`]. Documented on [`CONDITION_SLOT`].
fn is_truthy(v: &SlotValue) -> bool {
    match v {
        SlotValue::Null => false,
        SlotValue::Bool(b) => *b,
        SlotValue::Int(n) => *n != 0,
        SlotValue::Float(f) => *f != 0.0 && !f.is_nan(),
        SlotValue::String(s) => !s.is_empty(),
        SlotValue::Bytes(b) => !b.is_empty(),
        SlotValue::Json(j) => match j {
            serde_json::Value::Null => false,
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
            serde_json::Value::String(s) => !s.is_empty(),
            serde_json::Value::Array(a) => !a.is_empty(),
            serde_json::Value::Object(o) => !o.is_empty(),
        },
        // SlotValue is #[non_exhaustive]; unknown future variants are
        // treated as falsy. A `branch` deliberately refuses to guess.
        _ => false,
    }
}

#[async_trait]
impl NodeBehavior for Branch {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let condition = input
            .remove(CONDITION_SLOT)
            .ok_or_else(|| BranchError::MissingCondition.into_node_error())?;
        let value = input
            .remove(VALUE_SLOT)
            .ok_or_else(|| BranchError::MissingValue.into_node_error())?;

        let taken = is_truthy(&condition);

        let span = tracing::info_span!(
            "branch.invoke",
            node_id = %ctx.node,
            run_id = %ctx.run,
            taken = if taken { "true" } else { "false" },
            cancel_observed = tracing::field::Empty,
        );
        let _enter = span.enter();

        if ctx.cancel.is_cancelled() {
            span.record("cancel_observed", true);
            return Err(NodeError::Cancelled);
        }
        span.record("cancel_observed", false);

        let mut out = SlotMap::new();
        if taken {
            out.insert(TRUE_OUT_SLOT.to_owned(), value);
        } else {
            out.insert(FALSE_OUT_SLOT.to_owned(), value);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::Future;
    use std::pin::Pin;

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

    fn build_input(condition: SlotValue, value: SlotValue) -> SlotMap {
        let mut m = SlotMap::new();
        m.insert(CONDITION_SLOT.to_owned(), condition);
        m.insert(VALUE_SLOT.to_owned(), value);
        m
    }

    async fn invoke(condition: SlotValue, value: SlotValue) -> SlotMap {
        let body = Branch::new();
        let node = NodeId::new("test.branch").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        body.invoke(ctx, build_input(condition, value))
            .await
            .expect("invoke")
    }

    #[tokio::test]
    async fn bool_true_takes_true_branch() {
        let out = invoke(SlotValue::Bool(true), SlotValue::String("payload".into())).await;
        assert!(matches!(out.get(TRUE_OUT_SLOT), Some(SlotValue::String(s)) if s == "payload"));
        assert!(!out.contains_key(FALSE_OUT_SLOT));
    }

    #[tokio::test]
    async fn bool_false_takes_false_branch() {
        let out = invoke(SlotValue::Bool(false), SlotValue::Int(42)).await;
        assert!(matches!(out.get(FALSE_OUT_SLOT), Some(SlotValue::Int(42))));
        assert!(!out.contains_key(TRUE_OUT_SLOT));
    }

    #[tokio::test]
    async fn zero_int_is_falsy() {
        let out = invoke(SlotValue::Int(0), SlotValue::Null).await;
        assert!(out.contains_key(FALSE_OUT_SLOT));
    }

    #[tokio::test]
    async fn nonzero_int_is_truthy() {
        let out = invoke(SlotValue::Int(-1), SlotValue::Null).await;
        assert!(out.contains_key(TRUE_OUT_SLOT));
    }

    #[tokio::test]
    async fn empty_string_is_falsy() {
        let out = invoke(SlotValue::String(String::new()), SlotValue::Null).await;
        assert!(out.contains_key(FALSE_OUT_SLOT));
    }

    #[tokio::test]
    async fn null_is_falsy() {
        let out = invoke(SlotValue::Null, SlotValue::Null).await;
        assert!(out.contains_key(FALSE_OUT_SLOT));
    }

    #[tokio::test]
    async fn json_object_truthy_when_non_empty() {
        let truthy = invoke(
            SlotValue::Json(serde_json::json!({"k": 1})),
            SlotValue::Null,
        )
        .await;
        assert!(truthy.contains_key(TRUE_OUT_SLOT));
        let falsy = invoke(SlotValue::Json(serde_json::json!({})), SlotValue::Null).await;
        assert!(falsy.contains_key(FALSE_OUT_SLOT));
    }

    #[tokio::test]
    async fn missing_condition_is_error() {
        let body = Branch::new();
        let node = NodeId::new("test.branch").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = SlotMap::new();
        input.insert(VALUE_SLOT.to_owned(), SlotValue::Null);
        let err = body.invoke(ctx, input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains(CONDITION_SLOT), "{msg}");
    }

    #[tokio::test]
    async fn missing_value_is_error() {
        let body = Branch::new();
        let node = NodeId::new("test.branch").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        let mut input = SlotMap::new();
        input.insert(CONDITION_SLOT.to_owned(), SlotValue::Bool(true));
        let err = body.invoke(ctx, input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains(VALUE_SLOT), "{msg}");
    }

    #[tokio::test]
    async fn cancelled_run_surfaces_cancelled_error() {
        let body = Branch::new();
        let node = NodeId::new("test.branch").unwrap();
        let cancel = AlreadyCancelled;
        let ctx = make_ctx(&node, &cancel);
        let input = build_input(SlotValue::Bool(true), SlotValue::Null);
        let err = body.invoke(ctx, input).await.expect_err("must error");
        assert!(matches!(err, NodeError::Cancelled));
    }
}
