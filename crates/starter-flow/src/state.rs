//! Engine-typed `RunState` per R6 — the simplification that dissolved
//! the adk-rust checkpoint blob.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" / "What lands in
//! `starter-flow`" — the strongly-typed, engine-owned run state that
//! replaces the opaque checkpoint payload from adk-rust. Serialized
//! by `run` on Pause / Stopped; never exposed across the SPI seam.
//!
//! Phase-1 marker: empty — populated in Phase 2.
