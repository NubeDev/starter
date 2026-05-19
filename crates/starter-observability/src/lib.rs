//! # starter-observability
//!
//! Building blocks every starter-based binary uses to set up logging,
//! metrics, and request middleware. Not a server — middleware is
//! returned as `tower::Layer` impls and mounted by `starter-server`
//! (or any other tower-compatible transport).
//!
//! Three sub-modules, one job each:
//!
//! - [`tracing`] — initialise `tracing-subscriber` with sensible
//!   defaults.
//! - [`metrics`] — create a prometheus `Registry` and standard
//!   counters/histograms.
//! - [`middleware`] — request-id and latency middleware factories.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod metrics;
pub mod middleware;
pub mod tracing;
