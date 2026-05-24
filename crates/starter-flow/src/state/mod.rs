//! Engine-typed `RunState` per R6 and the in-process
//! [`NodeStateStore`](starter_flow_spi::state::NodeStateStore) impl.
//!
//! The original [`run_state`] module is the engine-typed replacement
//! for adk-rust's checkpoint blob; everything declared there is
//! re-exported from this module so `crate::state::RunState` continues
//! to resolve unchanged.
//!
//! [`in_memory::InMemoryNodeStateStore`] is the bare-engine implementation
//! of the per-node persistent state SPI seam introduced in stage A+B.1
//! (`DOCS/flow/scope/node-state.md`). It is the default the in-process
//! engine wires when no SQLite-backed store is attached, and it is what
//! every unit test of a stateful node reaches for. The sister sqlite
//! impl lives in `starter-store-sqlite/src/flow/node_state.rs`; both
//! run the same parameterised matrix.

pub mod in_memory;
mod run_state;

pub use run_state::*;
