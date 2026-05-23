//! `rubix-spi` — service-provider interface for the rubix tree.
//!
//! Phase 0 reserves the module slots so downstream crates can refer
//! to them via stable paths even before any item lands. Each slot is
//! intentionally empty; the corresponding phase fills it in:
//!
//! - `kind_manifest` — Phase 1 (see `docs/design/KIND-MANIFEST.md`).
//! - `msg`           — Phase 1 (everything-as-node message envelope).
//! - `slot_schema`   — Phase 1 (slot wiring on the graph).
//! - `dto`           — Phase 1 (transport-facing DTOs).
//! - `artifacts`     — Phase 4 (warehouse mart artifacts).

#![forbid(unsafe_code)]

pub mod artifacts {}
pub mod dto {}
pub mod kind_manifest {}
pub mod msg {}
pub mod slot_schema {}
