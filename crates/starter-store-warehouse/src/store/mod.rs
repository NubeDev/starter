//! Typed write paths. **Raw `INSERT` strings outside this crate
//! are forbidden** per W8 — every history-table write goes through
//! one of the modules below, which means the
//! `async_insert=1 / wait_for_async_insert=1` discipline is
//! applied centrally (in `ChClient::connect`) and cannot drift.
//!
//! The lint enforcement lives in this crate's CI config; the
//! review-gate rule is "if you find `INSERT INTO (raw_events|
//! samples|events|documents)` in a grep outside `src/store/`,
//! reject the PR."

pub mod documents;
pub mod events;
pub mod raw_events;
pub mod samples;
