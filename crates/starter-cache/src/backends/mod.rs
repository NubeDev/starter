//! Concrete cache backends. Each one is gated behind its own cargo
//! feature so consumers pay only for what they wire in.
//!
//! - [`moka`] — in-process TinyLFU cache (default, feature `moka`).
//! - [`noop`] — always-miss cache; useful for tests and for wiring
//!   "cache disabled" without touching call sites.
//!
//! Future: `valkey` (BSD-3 Redis fork) for cross-instance sharing.

#[cfg(feature = "moka")]
pub mod moka;

pub mod noop;
