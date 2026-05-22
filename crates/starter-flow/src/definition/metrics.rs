//! Per-engine definition-layer counters per
//! `DOCS/flow/scope/hot-reload.md` Observability section.
//!
//! Mirrors [`crate::metrics::RunMetricsCell`] in shape:
//! lock-free `AtomicU64`s for the cumulative counters, plus a
//! `len()` derived gauge for the active-topologies surface. Hosts
//! that emit Prometheus take a [`DefinitionMetrics`] snapshot via
//! [`DefinitionMetricsCell::snapshot`] and re-export under the
//! names the spec lists.
//!
//! Counter labels match the spec:
//! - `flow_definition_publishes_total{outcome="published"}` \u2014
//!   [`Self::add_published`].
//! - `flow_definition_publishes_total{outcome="short_circuited"}` \u2014
//!   [`Self::add_short_circuited`].
//! - `flow_definition_publishes_total{outcome="rejected"}` \u2014
//!   [`Self::add_rejected`].
//! - `flow_definition_swaps_total` \u2014 [`Self::add_swap`]. Initial
//!   mounts count toward this counter (a first-time mount is a
//!   swap-from-None).
//! - `flow_definition_resolve_failures_total` \u2014
//!   [`Self::add_resolve_failure`]. Bumped alongside the rejection
//!   counter when the rejection cause was a resolver error
//!   (settings / kind / link \u2014 not BodyShape).
//! - `flow_definition_active_topologies` \u2014 read live from
//!   [`crate::definition::active::ActiveTopologies::len`] when the
//!   host samples; the cell does not duplicate that count.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Live definition-layer counters. One per [`crate::definition::DefinitionManager`].
#[derive(Debug, Default)]
pub struct DefinitionMetricsCell {
    publishes_published: AtomicU64,
    publishes_short_circuited: AtomicU64,
    publishes_rejected: AtomicU64,
    swaps: AtomicU64,
    resolve_failures: AtomicU64,
}

/// Flat snapshot returned by [`DefinitionMetricsCell::snapshot`].
///
/// `#[non_exhaustive]` so new counters can be added without a
/// breaking change to the consumer surface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DefinitionMetrics {
    /// `flow_definition_publishes_total{outcome="published"}`
    pub publishes_published: u64,
    /// `flow_definition_publishes_total{outcome="short_circuited"}`
    pub publishes_short_circuited: u64,
    /// `flow_definition_publishes_total{outcome="rejected"}`
    pub publishes_rejected: u64,
    /// `flow_definition_swaps_total`
    pub swaps: u64,
    /// `flow_definition_resolve_failures_total`
    pub resolve_failures: u64,
}

impl DefinitionMetricsCell {
    /// Construct a fresh, zeroed cell wrapped in an `Arc`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Snapshot all counters into [`DefinitionMetrics`].
    pub fn snapshot(&self) -> DefinitionMetrics {
        DefinitionMetrics {
            publishes_published: self.publishes_published.load(Ordering::Acquire),
            publishes_short_circuited: self.publishes_short_circuited.load(Ordering::Acquire),
            publishes_rejected: self.publishes_rejected.load(Ordering::Acquire),
            swaps: self.swaps.load(Ordering::Acquire),
            resolve_failures: self.resolve_failures.load(Ordering::Acquire),
        }
    }

    /// `flow_definition_publishes_total{outcome="published"}`++.
    pub fn add_published(&self) {
        self.publishes_published.fetch_add(1, Ordering::AcqRel);
    }

    /// `flow_definition_publishes_total{outcome="short_circuited"}`++.
    pub fn add_short_circuited(&self) {
        self.publishes_short_circuited
            .fetch_add(1, Ordering::AcqRel);
    }

    /// `flow_definition_publishes_total{outcome="rejected"}`++.
    pub fn add_rejected(&self) {
        self.publishes_rejected.fetch_add(1, Ordering::AcqRel);
    }

    /// `flow_definition_swaps_total`++.
    pub fn add_swap(&self) {
        self.swaps.fetch_add(1, Ordering::AcqRel);
    }

    /// `flow_definition_resolve_failures_total`++.
    pub fn add_resolve_failure(&self) {
        self.resolve_failures.fetch_add(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_all_counters() {
        let c = DefinitionMetricsCell::new();
        c.add_published();
        c.add_published();
        c.add_short_circuited();
        c.add_rejected();
        c.add_swap();
        c.add_swap();
        c.add_swap();
        c.add_resolve_failure();
        let s = c.snapshot();
        assert_eq!(s.publishes_published, 2);
        assert_eq!(s.publishes_short_circuited, 1);
        assert_eq!(s.publishes_rejected, 1);
        assert_eq!(s.swaps, 3);
        assert_eq!(s.resolve_failures, 1);
    }
}
