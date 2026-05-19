//! Engine state machine per R12:
//! Starting → Running → Pausing → Paused → Resuming → Stopping → Stopped.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" — owns the
//! state-machine transitions, the propagator handle, and the
//! `Cancel` token wiring that the three-level stop (R6 + R12)
//! drives.
//!
//! Phase-1 marker: empty — populated in Phase 2.
