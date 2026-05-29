//! # starter-windowed
//!
//! Engine-agnostic windowed delta-fetch. The crate carries no
//! dependency on `starter-cache` — it is a **read-shape** primitive,
//! not a storage-shape one. Any caller that wants windowed
//! delta-fetch without caching (flow nodes, agent steps, export
//! jobs, CLI tools) can use it directly.
//!
//! The module shape is pinned by the [opt-in caching proposal][1]
//! `Companion crate — starter-windowed` section:
//!
//! - [`spec::WindowedSpec`] — declarative shape (bucket size,
//!   alignment, tail/body TTLs).
//! - [`bucket::snap_to_bucket`] / [`bucket::decompose`] — math.
//! - [`fetch::WindowedFetcher`] — the trait per-engine impls
//!   implement (Timescale, Postgres, in-memory mock, …).
//! - [`stitch::Stitchable`] — combine `Vec<T>` into one `T`.
//! - [`delta::extend`] — given a cached range and a requested range,
//!   return only the missing sub-ranges to fetch.
//!
//! Per-engine impls live in `starter-store-warehouse` and
//! `starter-store-postgres` so this crate has zero engine deps.
//!
//! [1]: ../../../rubix/docs/proposal/fe-cache-opt-in.md

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bucket;
pub mod delta;
pub mod fetch;
pub mod spec;
pub mod stitch;

pub use bucket::{decompose, snap_to_bucket, Bucket};
pub use delta::{extend, TimeRange};
pub use fetch::{FetchError, WindowedFetcher};
pub use spec::{AlignTo, WindowedSpec};
pub use stitch::{RowSet, Stitchable};
