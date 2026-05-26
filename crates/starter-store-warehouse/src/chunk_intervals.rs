//! TimescaleDB `chunk_time_interval` constants per L1/L2 hypertable.
//!
//! Sizing rationale (proposal §"Chunk sizing"):
//!
//! * **L1 (`raw_events`)** — monthly. Raw event volume is large;
//!   one chunk per month keeps compression and retention drops
//!   cheap while keeping per-chunk row counts in the Timescale-
//!   recommended sweet spot (~25M rows).
//! * **L2 (`samples`, `events`, `documents`)** — weekly. L2 has
//!   ~30× fewer rows than L1 after cleaning so weekly granularity
//!   keeps per-chunk row counts comparable to L1, and the cagg
//!   refresh windows align with operator-visible cadences.

/// `chunk_time_interval` for the L1 hypertable (`raw_events`).
pub const L1_CHUNK_INTERVAL: &str = "1 month";

/// `chunk_time_interval` for every L2 hypertable. Per the proposal
/// all three L2 tables share the same cadence.
pub const L2_CHUNK_INTERVAL: &str = "1 week";

/// L2 cadence as it applies to `samples`. Identical to
/// [`L2_CHUNK_INTERVAL`]; kept as a named constant so any future
/// per-table divergence has a stable callsite.
pub const L2_SAMPLES_CHUNK_INTERVAL: &str = L2_CHUNK_INTERVAL;

/// L2 cadence as it applies to `events`.
pub const L2_EVENTS_CHUNK_INTERVAL: &str = L2_CHUNK_INTERVAL;

/// L2 cadence as it applies to `documents`.
pub const L2_DOCUMENTS_CHUNK_INTERVAL: &str = L2_CHUNK_INTERVAL;
