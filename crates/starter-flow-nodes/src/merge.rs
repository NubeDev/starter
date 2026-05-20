//! `merge` — fan-in join node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R1 — Everything
//! is a Node" (the `join` half of the parallel-merge example in § "R7
//! — The AI agent is a node kind, not a runtime") and scheduled in
//! § "Phase 5 — Remaining built-in node kinds". Combines whatever
//! input slots the propagator delivers this invocation into a single
//! [`MERGED_SLOT`] output. The wait / firing policy (all vs any vs
//! first) is engine-owned per § "R3 — The engine is a reader of
//! policies, never an owner"; this body just consolidates what it
//! receives.
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** `merge` is a plain
//!   [`NodeBehavior`] impl.
//! - **R2 — One write chokepoint.** The body returns one output
//!   [`SlotMap`]; the propagator routes through
//!   `GraphStore::write_slot`.
//! - **R3 — Engine reads policies.** Combine *strategy* is
//!   configurable via [`STRATEGY_SLOT`]; *waiting* policy (when the
//!   propagator chooses to invoke) is a separate, engine-level
//!   policy and stays out of this body.
//! - **R5 — Stateless behaviours.** Unit struct.
//! - **R10 — Reverse-DNS ids.** [`KIND_ID`] is verbatim under
//!   `starter.flow.*`.
//! - **R12 observability.** Every invocation opens a `merge.invoke`
//!   tracing span recording `(node_id, run_id, strategy, inputs,
//!   cancel_observed)`.
//! - **R13 cancellation.** Sync work — one
//!   `ctx.cancel.is_cancelled()` check before emitting.

use async_trait::async_trait;
use serde_json::{Map as JsonMap, Value as JsonValue};
use thiserror::Error;

use starter_flow_spi::node::{
    anyhow_compat, KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.merge";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub const DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.merge.label",
        "starter.flow.node.merge.summary",
        "starter.flow.node.merge.help",
    );

/// Optional config slot naming the combine strategy. One of
/// `{"object", "array", "first"}`. Defaults to [`Strategy::Object`].
pub const STRATEGY_SLOT: &str = "strategy";

/// Output slot the consolidated value is written to. Shape depends on
/// the [`Strategy`]:
///
/// - `Object` — `SlotValue::Json(object)` keyed by input slot name.
/// - `Array`  — `SlotValue::Json(array)` of input values in the
///   propagator's delivery order (which is `SlotMap`'s
///   lexicographic key order today).
/// - `First`  — the single input value verbatim (no wrapping).
pub const MERGED_SLOT: &str = "merged";

/// How the body combines its inputs. Wait / firing policy stays on
/// the engine; this enum is purely the *combine* side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Emit a JSON object keyed by input slot name.
    Object,
    /// Emit a JSON array of input values in delivery order.
    Array,
    /// Emit the single first input value verbatim. Multiple inputs
    /// are an error.
    First,
}

impl Strategy {
    fn parse(s: &str) -> Result<Self, MergeError> {
        match s {
            "object" => Ok(Self::Object),
            "array" => Ok(Self::Array),
            "first" => Ok(Self::First),
            other => Err(MergeError::InvalidStrategy(other.to_owned())),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Array => "array",
            Self::First => "first",
        }
    }
}

/// Typed errors surfaced by [`Merge::invoke`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MergeError {
    /// The body was invoked with zero input slots. The engine's wait
    /// policy is responsible for not firing this case; if it does
    /// fire, surface a typed error rather than emit a malformed slot.
    #[error("merge invoked with empty input")]
    EmptyInput,

    /// [`STRATEGY_SLOT`] was present but not [`SlotValue::String`].
    #[error("merge `{STRATEGY_SLOT}` must be SlotValue::String")]
    InvalidStrategyType,

    /// `strategy` was a string but not one of `object|array|first`.
    #[error("merge `{STRATEGY_SLOT}` must be object|array|first; got `{0}`")]
    InvalidStrategy(String),

    /// `Strategy::First` got more than one input slot.
    #[error("merge strategy `first` requires exactly one input, got {0}")]
    FirstNeedsOneInput(usize),
}

impl MergeError {
    fn into_node_error(self) -> NodeError {
        NodeError::Other(anyhow_compat::Error(Box::new(self)))
    }
}

/// `merge` node-kind behaviour. Stateless (R5) — unit struct.
pub struct Merge {
    kind: KindId,
}

impl Default for Merge {
    fn default() -> Self {
        Self::new()
    }
}

impl Merge {
    /// Construct a [`Merge`] node body. Panics if [`KIND_ID`] is not
    /// a valid reverse-DNS identifier (compile-time invariant).
    pub fn new() -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
        }
    }
}

/// [`SlotValue`] → [`JsonValue`] projection used by the object/array
/// strategies. `Bytes` collapses to a JSON array of byte values so the
/// downstream wire shape stays self-describing without pulling base64.
fn slot_to_json(v: SlotValue) -> JsonValue {
    match v {
        SlotValue::Null => JsonValue::Null,
        SlotValue::Bool(b) => JsonValue::Bool(b),
        SlotValue::Int(n) => JsonValue::from(n),
        SlotValue::Float(f) => serde_json::Number::from_f64(f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        SlotValue::String(s) => JsonValue::String(s),
        SlotValue::Bytes(b) => JsonValue::Array(b.into_iter().map(JsonValue::from).collect()),
        SlotValue::Json(j) => j,
        // Future SlotValue variants — preserve as a Debug string
        // rather than silently lose the value.
        other => JsonValue::String(format!("{other:?}")),
    }
}

#[async_trait]
impl NodeBehavior for Merge {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let strategy = match input.remove(STRATEGY_SLOT) {
            None => Strategy::Object,
            Some(SlotValue::String(s)) => Strategy::parse(&s).map_err(MergeError::into_node_error)?,
            Some(_) => return Err(MergeError::InvalidStrategyType.into_node_error()),
        };

        if input.is_empty() {
            return Err(MergeError::EmptyInput.into_node_error());
        }

        let span = tracing::info_span!(
            "merge.invoke",
            node_id = %ctx.node,
            run_id = %ctx.run,
            strategy = strategy.as_str(),
            inputs = input.len(),
            cancel_observed = tracing::field::Empty,
        );
        let _enter = span.enter();

        if ctx.cancel.is_cancelled() {
            span.record("cancel_observed", true);
            return Err(NodeError::Cancelled);
        }
        span.record("cancel_observed", false);

        let merged = match strategy {
            Strategy::Object => {
                let mut obj = JsonMap::new();
                for (k, v) in input {
                    obj.insert(k, slot_to_json(v));
                }
                SlotValue::Json(JsonValue::Object(obj))
            }
            Strategy::Array => {
                // `SlotMap` is a `BTreeMap`, so iteration is lexicographic
                // — stable and reproducible across runs.
                let arr: Vec<JsonValue> = input.into_values().map(slot_to_json).collect();
                SlotValue::Json(JsonValue::Array(arr))
            }
            Strategy::First => {
                if input.len() != 1 {
                    return Err(MergeError::FirstNeedsOneInput(input.len()).into_node_error());
                }
                // Safe: just checked len == 1.
                input.into_values().next().expect("len == 1")
            }
        };

        let mut out = SlotMap::new();
        out.insert(MERGED_SLOT.to_owned(), merged);
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

    fn make_ctx<'a>(node: &'a NodeId, cancel: &'a dyn Cancel) -> NodeCtx<'a> {
        NodeCtx::new(
            starter_flow_spi::flow::RunId::new(),
            node,
            cancel,
            starter_flow_spi::skill::SkillSelection::NONE,
        )
    }

    async fn invoke(input: SlotMap) -> Result<SlotMap, NodeError> {
        let body = Merge::new();
        let node = NodeId::new("test.merge").unwrap();
        let cancel = NoCancel;
        let ctx = make_ctx(&node, &cancel);
        body.invoke(ctx, input).await
    }

    #[tokio::test]
    async fn object_strategy_default_keys_by_slot_name() {
        let mut input = SlotMap::new();
        input.insert("a".into(), SlotValue::Int(1));
        input.insert("b".into(), SlotValue::String("two".into()));
        let out = invoke(input).await.expect("invoke");
        let merged = match out.get(MERGED_SLOT) {
            Some(SlotValue::Json(j)) => j.clone(),
            other => panic!("expected Json, got {other:?}"),
        };
        let obj = merged.as_object().expect("object");
        assert_eq!(obj.get("a"), Some(&JsonValue::from(1_i64)));
        assert_eq!(obj.get("b"), Some(&JsonValue::String("two".into())));
    }

    #[tokio::test]
    async fn array_strategy_emits_array_in_key_order() {
        let mut input = SlotMap::new();
        input.insert(STRATEGY_SLOT.into(), SlotValue::String("array".into()));
        input.insert("b".into(), SlotValue::Int(20));
        input.insert("a".into(), SlotValue::Int(10));
        let out = invoke(input).await.expect("invoke");
        let merged = match out.get(MERGED_SLOT) {
            Some(SlotValue::Json(j)) => j.clone(),
            other => panic!("expected Json, got {other:?}"),
        };
        let arr = merged.as_array().expect("array");
        assert_eq!(arr.len(), 2);
        // BTreeMap key order is lexicographic → a first, then b.
        assert_eq!(arr[0], JsonValue::from(10_i64));
        assert_eq!(arr[1], JsonValue::from(20_i64));
    }

    #[tokio::test]
    async fn first_strategy_passes_value_through() {
        let mut input = SlotMap::new();
        input.insert(STRATEGY_SLOT.into(), SlotValue::String("first".into()));
        input.insert("only".into(), SlotValue::Bool(true));
        let out = invoke(input).await.expect("invoke");
        assert!(matches!(out.get(MERGED_SLOT), Some(SlotValue::Bool(true))));
    }

    #[tokio::test]
    async fn first_strategy_rejects_multiple_inputs() {
        let mut input = SlotMap::new();
        input.insert(STRATEGY_SLOT.into(), SlotValue::String("first".into()));
        input.insert("a".into(), SlotValue::Int(1));
        input.insert("b".into(), SlotValue::Int(2));
        let err = invoke(input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains("exactly one input"), "{msg}");
    }

    #[tokio::test]
    async fn empty_input_is_error() {
        let err = invoke(SlotMap::new()).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains("empty input"), "{msg}");
    }

    #[tokio::test]
    async fn invalid_strategy_string_is_error() {
        let mut input = SlotMap::new();
        input.insert(STRATEGY_SLOT.into(), SlotValue::String("bogus".into()));
        input.insert("a".into(), SlotValue::Null);
        let err = invoke(input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains("object|array|first"), "{msg}");
    }

    #[tokio::test]
    async fn invalid_strategy_type_is_error() {
        let mut input = SlotMap::new();
        input.insert(STRATEGY_SLOT.into(), SlotValue::Int(1));
        input.insert("a".into(), SlotValue::Null);
        let err = invoke(input).await.expect_err("must error");
        let msg = format!("{err}");
        assert!(msg.contains("SlotValue::String"), "{msg}");
    }
}
