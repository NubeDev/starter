//! `Stitchable` — combine per-bucket fetches into a single result.

/// Result types that can be concatenated across buckets.
pub trait Stitchable: Sized + Send {
    /// Merge a vector of per-bucket parts (in chronological order)
    /// into a single result.
    fn stitch(parts: Vec<Self>) -> Self;
}

impl<U: Send> Stitchable for Vec<U> {
    fn stitch(parts: Vec<Self>) -> Self {
        let total: usize = parts.iter().map(|p| p.len()).sum();
        let mut out = Vec::with_capacity(total);
        for mut p in parts {
            out.append(&mut p);
        }
        out
    }
}

/// `RowSet` — engine-agnostic carrier for windowed query results.
///
/// JSON-row payloads are the lowest-common-denominator both per-engine
/// fetcher impls (`TimescaleWindowedFetcher`, `PgWindowedFetcher`)
/// already emit, and they stitch trivially. Specialised callers are
/// free to implement `Stitchable` on their own row type.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RowSet {
    /// One row per element. Shape is loader-defined.
    pub rows: Vec<serde_json::Value>,
}

impl RowSet {
    /// Construct a `RowSet` from a vector of rows.
    pub fn new(rows: Vec<serde_json::Value>) -> Self {
        Self { rows }
    }

    /// Borrow the rows.
    pub fn rows(&self) -> &[serde_json::Value] {
        &self.rows
    }
}

impl Stitchable for RowSet {
    fn stitch(parts: Vec<Self>) -> Self {
        let total: usize = parts.iter().map(|p| p.rows.len()).sum();
        let mut rows = Vec::with_capacity(total);
        for mut p in parts {
            rows.append(&mut p.rows);
        }
        Self { rows }
    }
}
