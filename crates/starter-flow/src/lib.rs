//! # starter-flow
//!
//! THE flow engine: slot propagator, single `write_slot` chokepoint
//! [`GraphStore`] impl, `NodeKindRegistry`, `FlowRegistry`, engine
//! state machine, run lifecycle, three-level stop.
//!
//! Phase 1 of `DOCS/flow/scope/SCOPE.md` ships this crate as a module
//! skeleton — every module declared below points at an empty file
//! whose only contents are a SCOPE pointer + Phase-N marker doc
//! comment. The engine itself lands in Phase 2 (graph + propagator +
//! engine + run + state) and Phase 3 (registry plumbing + persistence
//! hooks); see the SCOPE phasing block for the full breakdown.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Graph storage + the single `write_slot` chokepoint per R2.
/// Lands in Phase 2 — see SCOPE "Phase 2 — `starter-flow` engine".
pub mod graph;

/// `NodeKindRegistry` + `FlowRegistry` per the SCOPE
/// "What lands in `starter-flow`" crate block.
/// Lands in Phase 3 — see SCOPE phasing block.
pub mod registry;

/// Synchronous tokio propagator loop per R2 and the rubix
/// `live_wire.rs` Decisions reference.
/// Lands in Phase 2 — see SCOPE "Phase 2 — `starter-flow` engine".
pub mod propagator;

/// Engine state machine: Starting → Running → Pausing → Paused →
/// Resuming → Stopping → Stopped per R12.
/// Lands in Phase 2 — see SCOPE "Phase 2 — `starter-flow` engine".
pub mod engine;

/// Run lifecycle: `Cancel` plumbing, `RunState`, checkpointing per R6.
/// Lands in Phase 2 (lifecycle) and Phase 7 (three-level stop +
/// checkpoint persistence) — see SCOPE phasing block.
pub mod run;

/// Engine-typed `RunState` per R6 — the simplification that dissolved
/// the adk-rust checkpoint blob.
/// Lands in Phase 2 — see SCOPE "Phase 2 — `starter-flow` engine".
pub mod state;

/// Engine-level health handle (`EngineHealth::{Healthy, Degraded}`).
/// Lands in Phase 3 stage 6 — see SCOPE "durability hardening" /
/// D-F3.11.
pub mod health;

/// Per-run observability counters (`subscriber_lagged_count`,
/// `degraded_dropped_count`). Lands in Phase 3 stage 6 — see SCOPE
/// "durability hardening" / D-F3.10 + D-F3.11.
pub mod metrics;
