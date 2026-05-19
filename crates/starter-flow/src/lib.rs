//! # starter-flow
//!
//! THE flow engine: slot propagator, single `write_slot` chokepoint
//! [`GraphStore`] impl, `NodeKindRegistry`, `FlowRegistry`, engine
//! state machine, run lifecycle, three-level stop.
//!
//! Phase 1 of `DOCS/flow/scope/SCOPE.md` ships this crate as an empty
//! skeleton so the workspace builds end-to-end. The engine itself
//! lands in Phase 2 — see the SCOPE "Phase 2 — `starter-flow` engine"
//! block for the full list of items that land then.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
