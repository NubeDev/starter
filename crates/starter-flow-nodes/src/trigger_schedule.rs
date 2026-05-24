//! `trigger.schedule` — cron-fired entry node.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R3 — The engine
//! is a reader of policies, never an owner" (the `trigger` policy
//! variant `schedule(cron)`) and § "R1 — Everything is a Node" ("A
//! trigger (explicit, event-driven, scheduled, webhook) is a node").
//! Backed by the durable scheduler noted in § "What this scope is
//! *not*"; lands alongside the rest of the trigger family in
//! § "Phase 5 — Remaining built-in node kinds".
//!
//! ## Role
//!
//! `trigger.schedule` is a **passive** entry node. The actual firing
//! comes from the host-side durable cron scheduler ("Phase B tick" —
//! `starter-cron` + `starter_scheduled_flows` PG table) which polls
//! `next_run_at` and, when due, kicks the flow runner. This body's job
//! is only to surface the cron expression carried on the node's
//! settings into a slot so `FlowAsService` (and any other surface that
//! enumerates a flow's schedules) can pick it up without having to
//! re-parse the flow YAML.
//!
//! In other words: the *firing* is host-driven; the *expression* is
//! flow-defined and travels here.
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** `trigger.schedule` is just a
//!   `NodeBehavior` impl that copies its config slot into its output
//!   slot.
//! - **R2 — One write chokepoint.** The body returns its output
//!   [`SlotMap`]; the propagator funnels every value through
//!   `GraphStore::write_slot`. No direct slot writes.
//! - **R5 — Stateless behaviours.** Unit struct.
//! - **R10 — Reverse-DNS ids.** [`KIND_ID`] is verbatim under
//!   `starter.flow.*`.
//! - **R12 observability.** Every invocation opens a
//!   `trigger_schedule.invoke` tracing span recording `(node_id,
//!   run_id, cron_expr, cancel_observed)`.
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
pub const KIND_ID: &str = "starter.flow.trigger.schedule";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub static DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.trigger-schedule.label",
        "starter.flow.node.trigger-schedule.summary",
        "starter.flow.node.trigger-schedule.help",
    );

/// Mandatory config slot carrying the cron expression that drives the
/// host-side durable scheduler. Slot value must be a
/// [`SlotValue::String`]. The body forwards the value verbatim into
/// [`SCHEDULE_SLOT`]; expression *validity* is the scheduler's
/// concern (`starter-cron::next_fire`).
pub const CRON_EXPR_SLOT: &str = "cron_expr";

/// Output slot the body writes the cron expression into, as a
/// [`SlotValue::String`]. `FlowAsService` (and any other surface that
/// enumerates a flow's schedules) reads this slot to discover the
/// cron expression without having to re-parse the flow YAML.
pub const SCHEDULE_SLOT: &str = "schedule";

/// Publish-time configuration carried on a `trigger.schedule` node's
/// `settings:` field in a flow body. Per
/// [`DOCS/flow/scope/settings.md`](../../../DOCS/flow/scope/settings.md)
/// Phase S-4: the kind exposes a typed schema derived from this
/// struct via [`schemars`] so editor surfaces can validate drafts.
///
/// Runtime [`TriggerSchedule::invoke`] still reads
/// [`CRON_EXPR_SLOT`] from the input [`SlotMap`]; once
/// `TopologyResolver::resolve` lands (`DOCS/flow/scope/hot-reload.md`
/// HR5) it will project [`Self::cron_expr`] into that slot. Until
/// then this struct only powers schema-fetch surfaces.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TriggerScheduleSettings {
    /// Cron expression the host-side durable scheduler interprets to
    /// decide when to fire the flow (e.g. `"0 0 * * 0"` for weekly).
    /// Validation lives in `starter-cron::next_fire`; this struct
    /// only carries the raw string.
    pub cron_expr: String,
}

/// Derived JSON Schema for [`TriggerScheduleSettings`]. Returned by
/// reference from [`TriggerSchedule::config_schema`]; built once per
/// process via [`LazyLock`].
pub static TRIGGER_SCHEDULE_SETTINGS_SCHEMA: LazyLock<RootSchema> =
    LazyLock::new(|| schemars::schema_for!(TriggerScheduleSettings));

/// Typed errors surfaced by [`TriggerSchedule::invoke`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TriggerScheduleError {
    /// The input did not carry a string-valued [`CRON_EXPR_SLOT`].
    #[error(
        "trigger.schedule input missing `{CRON_EXPR_SLOT}` slot \
         (must be SlotValue::String carrying the cron expression)"
    )]
    MissingCronExpr,

    /// The [`CRON_EXPR_SLOT`] was present but not a
    /// [`SlotValue::String`].
    #[error("trigger.schedule `{CRON_EXPR_SLOT}` must be SlotValue::String")]
    InvalidCronExprType,
}

impl TriggerScheduleError {
    fn into_node_error(self) -> NodeError {
        NodeError::Other(anyhow_compat::Error(Box::new(self)))
    }
}

/// `trigger.schedule` node-kind behaviour. Stateless (R5) — unit
/// struct.
pub struct TriggerSchedule {
    kind: KindId,
}

impl Default for TriggerSchedule {
    fn default() -> Self {
        Self::new()
    }
}

impl TriggerSchedule {
    /// Construct a [`TriggerSchedule`] node body. Panics if
    /// [`KIND_ID`] is not a valid reverse-DNS identifier
    /// (compile-time invariant).
    pub fn new() -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
        }
    }
}

#[async_trait]
impl NodeBehavior for TriggerSchedule {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    fn config_schema(&self) -> &'static RootSchema {
        &TRIGGER_SCHEDULE_SETTINGS_SCHEMA
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let cron_expr = match input.remove(CRON_EXPR_SLOT) {
            None => return Err(TriggerScheduleError::MissingCronExpr.into_node_error()),
            Some(SlotValue::String(s)) => s,
            Some(_) => return Err(TriggerScheduleError::InvalidCronExprType.into_node_error()),
        };

        // R12 observability span enclosing the (sync) emit.
        let span = tracing::info_span!(
            "trigger_schedule.invoke",
            node_id = %ctx.node,
            run_id = %ctx.run,
            cron_expr = %cron_expr,
            cancel_observed = tracing::field::Empty,
        );
        let _enter = span.enter();

        // R13 — single cancel check before the (sync) emit. There is
        // no await point to `select!` against.
        if ctx.cancel.is_cancelled() {
            span.record("cancel_observed", true);
            return Err(NodeError::Cancelled);
        }
        span.record("cancel_observed", false);

        let mut out = SlotMap::new();
        out.insert(SCHEDULE_SLOT.to_owned(), SlotValue::String(cron_expr));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::Future;
    use std::pin::Pin;

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

    fn input_with(cron: &str) -> SlotMap {
        let mut m = SlotMap::new();
        m.insert(
            CRON_EXPR_SLOT.to_owned(),
            SlotValue::String(cron.to_owned()),
        );
        m
    }

    /// Happy path: the cron expression flows verbatim from the input
    /// config slot to the `schedule` output slot.
    #[tokio::test]
    async fn cron_expr_passes_through_to_schedule_slot() {
        let node_kind = TriggerSchedule::new();
        let node = NodeId::new("flow.test.ts").unwrap();
        let cancel = NoCancel;

        let out = node_kind
            .invoke(make_ctx(&node, &cancel), input_with("0 0 * * 0"))
            .await
            .expect("happy path must succeed");

        assert_eq!(out.len(), 1);
        match out.get(SCHEDULE_SLOT) {
            Some(SlotValue::String(s)) => assert_eq!(s, "0 0 * * 0"),
            other => panic!("expected SlotValue::String on `schedule`; got {other:?}"),
        }
    }

    /// Missing `cron_expr` slot returns
    /// `TriggerScheduleError::MissingCronExpr`.
    #[tokio::test]
    async fn missing_cron_expr_returns_typed_error() {
        let node_kind = TriggerSchedule::new();
        let node = NodeId::new("flow.test.ts").unwrap();
        let cancel = NoCancel;

        let err = node_kind
            .invoke(make_ctx(&node, &cancel), SlotMap::new())
            .await
            .expect_err("missing cron_expr must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other; got {err:?}");
        };
        let e = boxed
            .0
            .downcast::<TriggerScheduleError>()
            .expect("Other must wrap TriggerScheduleError");
        assert!(matches!(*e, TriggerScheduleError::MissingCronExpr));
    }

    /// Non-string `cron_expr` slot returns
    /// `TriggerScheduleError::InvalidCronExprType`.
    #[tokio::test]
    async fn non_string_cron_expr_returns_typed_error() {
        let node_kind = TriggerSchedule::new();
        let node = NodeId::new("flow.test.ts").unwrap();
        let cancel = NoCancel;

        let mut input = SlotMap::new();
        input.insert(CRON_EXPR_SLOT.to_owned(), SlotValue::Int(42));

        let err = node_kind
            .invoke(make_ctx(&node, &cancel), input)
            .await
            .expect_err("non-string cron_expr must surface as Err");

        let NodeError::Other(boxed) = err else {
            panic!("expected NodeError::Other; got {err:?}");
        };
        let e = boxed
            .0
            .downcast::<TriggerScheduleError>()
            .expect("Other must wrap TriggerScheduleError");
        assert!(matches!(*e, TriggerScheduleError::InvalidCronExprType));
    }

    /// Cancel observed pre-emit surfaces `NodeError::Cancelled`
    /// without emitting on the output slot.
    #[tokio::test]
    async fn already_cancelled_returns_cancelled() {
        let node_kind = TriggerSchedule::new();
        let node = NodeId::new("flow.test.ts").unwrap();
        let cancel = AlreadyCancelled;

        let err = node_kind
            .invoke(make_ctx(&node, &cancel), input_with("0 0 * * 0"))
            .await
            .expect_err("cancelled invocation must return Err");
        assert!(matches!(err, NodeError::Cancelled));
    }
}
