//! Graph storage + the single `write_slot` chokepoint per R2.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" / "What lands in
//! `starter-flow`" — the in-engine `GraphStore` impl that funnels
//! every slot write through one path so the propagator can observe
//! and re-fire subscribers (with the `replay: bool` opt-out per R2).
//!
//! Phase-1 marker: empty — populated in Phase 2.
