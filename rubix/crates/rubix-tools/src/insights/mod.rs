//! Insights goal — tool implementations.
//!
//! Four verbs: `rubix.insights.rule.{list, create, enable, disable}`.
//! Backed by a shared [`InsightsRuleStore`] trait; today the only
//! impl is [`store::InMemoryInsightsStore`]. A PG-backed adapter
//! is a tracked follow-up (see the agent's
//! [`registry.rs`](../../../rubix-agent/src/registry.rs) module
//! docs).
//!
//! `create` performs an idempotent upsert; the toggle verbs are
//! no-ops when the new state already matches the current one.
//! None of the verbs route through `ReversibleTool` today — the
//! UndoDispatcher wiring is tracked separately.

pub mod rule_create;
pub mod rule_list;
pub mod rule_toggle;
pub mod store;
