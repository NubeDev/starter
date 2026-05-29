//! Concrete cache backends. Each one is gated behind its own cargo
//! feature so consumers pay only for what they wire in.
//!
//! - [`moka`] — in-process TinyLFU cache (default, feature `moka`).
//! - [`noop`] — always-miss cache; useful for tests and for wiring
//!   "cache disabled" without touching call sites.
//! - [`valkey`] — shared-across-replicas cache (feature `valkey`),
//!   v3 wiring point for multi-node deployments.

#[cfg(feature = "moka")]
pub mod moka;

pub mod noop;

#[cfg(feature = "valkey")]
pub mod valkey;
