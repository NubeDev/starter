//! The alerting subsystem: scheduler, evaluator, state machine, and notifiers.
//!
//! The route handlers are CRUD; the system is here. The scheduler ticks, the
//! evaluator runs each due rule's guarded query and feeds the result to the pure
//! state machine, and a transition records an event and notifies the rule's
//! channels unless silenced. The state machine and the threshold comparison are
//! pure and unit-tested; the evaluator orchestrates them with the store and the
//! query path, keeping the engine free of any of this (R2).

pub mod compare;
pub mod condition;
pub mod evaluate;
pub mod notify;
pub mod policy;
pub mod reduce;
pub mod schedule;
pub mod template;
pub mod transition;
