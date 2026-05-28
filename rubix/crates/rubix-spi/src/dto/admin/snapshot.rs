//! Multiplexed envelope returned by `GET /api/v1/admin/registry`.
//!
//! One [`Page<RegistryItem>`](starter_spi::paging::Page) per
//! requested kind, keyed by [`RegistryKind`]. Kinds the caller did
//! not request are absent from the map; the per-kind URL sugar
//! (`/admin/registry/tools` etc.) emits a single-entry snapshot.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use starter_spi::paging::Page;
use utoipa::ToSchema;

use super::item::RegistryItem;
use super::kind::RegistryKind;

/// One page per requested registry kind. `BTreeMap` keeps the wire
/// order deterministic — clients can switch on key without a sort
/// step.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct RegistrySnapshot {
    /// The per-kind pages. Wire shape:
    /// `{ "tool": {...}, "node": {...} }`. A kind the request did
    /// not name is absent (not `null`).
    pub kinds: BTreeMap<RegistryKind, Page<RegistryItem>>,
}

impl RegistrySnapshot {
    /// Empty snapshot.
    pub fn new() -> Self {
        Self {
            kinds: BTreeMap::new(),
        }
    }

    /// Insert a kind's page. Replaces any previous entry under the
    /// same kind.
    pub fn insert(&mut self, kind: RegistryKind, page: Page<RegistryItem>) {
        self.kinds.insert(kind, page);
    }

    /// `true` when no kinds carry a page.
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}

impl Default for RegistrySnapshot {
    fn default() -> Self {
        Self::new()
    }
}
