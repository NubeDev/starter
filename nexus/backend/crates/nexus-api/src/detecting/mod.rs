//! Detection runner (WS-15): the scheduled analytic engine that turns a stored
//! insight + a query into persistent findings.
//!
//! Sibling of [`crate::alerting`], by design. The scheduler is a near-copy of
//! the alert scheduler; the per-item body runs the insight over the query frame
//! and upserts findings instead of reducing to a scalar and notifying. Keeping
//! it a parallel module (rather than forking a shared scaffold) is the lower-risk
//! v1 choice the WS calls for.

pub mod run;
pub mod schedule;
