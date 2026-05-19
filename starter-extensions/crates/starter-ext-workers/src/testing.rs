//! Testing seam re-exports.
//!
//! [`WorkersSchedulerHandle::tick_now`] is the only deterministic
//! seam the workers adapter ships. It lives on the regular handle
//! (not behind `#[cfg(test)]`) because the admin layer wants the
//! same entry-point for an operator-triggered "run now" button.
//! The `pub` API is duplicated here as a one-line re-export so test
//! code can find it under the `starter_ext_workers::testing::` path
//! that mirrors other crates in the workspace.

pub use crate::scheduler::WorkersSchedulerHandle;
