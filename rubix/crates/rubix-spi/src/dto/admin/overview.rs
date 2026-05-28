//! Cheap counts surface — `GET /api/v1/admin/overview`.
//!
//! Returns the per-kind item count without paginating any list.
//! Lets the admin console render the landing dashboard without
//! issuing seven list calls.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::kind::RegistryKind;

/// Per-kind item counts. Always emits an entry for every
/// [`RegistryKind`] — a registry that holds zero items is reported
/// as `0`, not omitted, so clients can render a grid without a
/// presence check.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegistryOverview {
    /// Per-kind count of items the projection would emit. Sums to
    /// the same total a caller would observe by walking every
    /// `/admin/registry/<kind>` page.
    pub counts: BTreeMap<RegistryKind, u32>,
}

impl RegistryOverview {
    /// Build an overview where every kind reads from the supplied
    /// closure. Saves the caller from worrying about kind coverage
    /// — every [`RegistryKind`] is hit exactly once.
    pub fn from_fn<F>(mut count: F) -> Self
    where
        F: FnMut(RegistryKind) -> u32,
    {
        let mut counts = BTreeMap::new();
        for kind in RegistryKind::ALL.iter().copied() {
            counts.insert(kind, count(kind));
        }
        Self { counts }
    }
}
