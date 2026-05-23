//! Re-export of the W11 freshness envelope from
//! `starter-store-clickhouse`. The warehouse crate owns the
//! envelope shape in the read responses but does not duplicate
//! the probe — the probe lives next to the dictionary it queries.

pub use starter_store_clickhouse::dim_freshness::{
    DictFreshness, FreshnessProbe, Status,
};

/// The W11 `dimension_freshness` block as it appears at the top
/// of every read envelope: a map keyed by dictionary name.
#[derive(Clone, Debug, serde::Serialize)]
pub struct DimensionFreshness {
    pub entities_dict: DictFreshness,
}

impl DimensionFreshness {
    /// Convenience constructor — the only dictionary the
    /// warehouse owns today is `entities_dict`. Future dictionaries
    /// (e.g. `entity_refs_dict`) are added here without breaking
    /// the JSON shape because the envelope key is fixed by name.
    pub fn new(entities_dict: DictFreshness) -> Self {
        Self { entities_dict }
    }
}
