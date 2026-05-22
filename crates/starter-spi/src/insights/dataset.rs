//! [`Dataset`] + [`VecDatasetRows`] — the typed-rows currency every
//! derivation rule emits (Insights SCOPE D1).

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::coverage::Coverage;
use super::time::{TimeZoneId, Window};

/// Typed column metadata for a [`Dataset`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DatasetSchema {
    /// Column names, in left-to-right wire order. Phase 1 carries
    /// names only; later phases attach `Unit` / type metadata.
    pub columns: Vec<String>,
}

impl DatasetSchema {
    /// Construct a schema from column names.
    pub fn new<I: IntoIterator<Item = S>, S: Into<String>>(columns: I) -> Self {
        Self {
            columns: columns.into_iter().map(Into::into).collect(),
        }
    }
}

/// Streamable rows. Phase 1 ships `VecDatasetRows`; later phases
/// add `StreamingDatasetRows` (in `starter-insights`) for packs
/// that need to stream beyond the ~10k-rows / ~1MB cap.
pub trait DatasetRows: Send + Sync + std::fmt::Debug {
    /// Snapshot every row as `serde_json::Value`s, one per row.
    /// Bounded by the implementation; small-dataset impls return
    /// the entire vector. Streaming impls (Phase 3) materialise a
    /// bounded chunk.
    fn snapshot(&self) -> Vec<serde_json::Value>;

    /// Row count without materialising the values.
    fn len(&self) -> usize;

    /// Whether the row set is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// In-memory `DatasetRows` impl backed by a `Vec`. Suitable for
/// assertion packs with tiny evidence rows and small-dataset
/// derivation packs (rough cap: ~10k rows / ~1MB per D1).
#[derive(Debug, Clone)]
pub struct VecDatasetRows(Vec<serde_json::Value>);

impl VecDatasetRows {
    /// Wrap an owned `Vec`.
    pub fn new(rows: Vec<serde_json::Value>) -> Self {
        Self(rows)
    }

    /// Empty constructor.
    pub fn empty() -> Self {
        Self(Vec::new())
    }
}

impl DatasetRows for VecDatasetRows {
    fn snapshot(&self) -> Vec<serde_json::Value> {
        self.0.clone()
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

/// A `Dataset` is the derivation rule's output shape (R-ins-7).
///
/// `rows` is an `Arc<dyn DatasetRows>` so packs returning tiny
/// evidence rows can stay on `VecDatasetRows` (depending on
/// `starter-spi` only); packs streaming larger data depend on
/// `starter-insights` for `StreamingDatasetRows` (D1).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Dataset {
    /// Typed column metadata.
    pub schema: DatasetSchema,
    /// Row payload — streamable, bounded.
    pub rows: Arc<dyn DatasetRows>,
    /// First-class coverage, propagated through every derivation.
    pub coverage: Coverage,
    /// Time zone the dataset was computed against. Mandatory per
    /// R-ins-6 — DST-sensitive analysis fails without it.
    pub tz: TimeZoneId,
    /// Optional time window. `None` for point-in-time / static
    /// datasets.
    pub window: Option<Window>,
}

impl Dataset {
    /// Construct an empty point-in-time [`Dataset`] — used by tests
    /// and by Phase 1 IoT rules that emit a verdict with no
    /// dataset payload.
    pub fn empty_point(tz: TimeZoneId) -> Self {
        Self {
            schema: DatasetSchema::new(Vec::<String>::new()),
            rows: Arc::new(VecDatasetRows::empty()),
            coverage: Coverage::full_point(),
            tz,
            window: None,
        }
    }
}
