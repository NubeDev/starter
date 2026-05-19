//! Run lifecycle: `Cancel` plumbing + `RunState` + checkpointing per R6.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" (lifecycle +
//! Cancel propagation) and "Phase 7 — three-level stop" (checkpoint
//! persistence on Pause / Stopped). Owns the per-`RunId` handle the
//! engine hands back to callers and the checkpoint serializer that
//! writes through `RunStore`.
//!
//! Phase-1 marker: empty — populated in Phase 2 (lifecycle) and
//! Phase 7 (checkpointing).
