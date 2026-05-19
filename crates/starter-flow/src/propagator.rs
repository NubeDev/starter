//! Synchronous tokio propagator loop per R2 and the rubix
//! `live_wire.rs` Decisions reference.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" — the single
//! synchronous loop that drains the write queue, fires subscribers,
//! and re-enqueues downstream writes until quiescent. Awaits node
//! `invoke` futures inline so ordering is deterministic per slot.
//!
//! Phase-1 marker: empty — populated in Phase 2.
