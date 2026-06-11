//! Zenoh datasource store concerns: the pre-save connect probe.
//!
//! The catalogue declares `zenoh` as a stream connector with a `connect` test
//! mode; this module supplies that probe, gated behind the crate's `zenoh`
//! feature so a default build pulls none of zenoh's transitive deps.

mod probe;

pub use probe::{probe, ProbeParams};
