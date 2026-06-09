//! Tag domain records and the inputs to set them.
//!
//! Store-layer types, distinct from the wire DTOs in `nexus-spi`. A tag is a
//! `key` with an optional `value`: `value = None` is a bare label, `Some` is a
//! key:value pair.

/// A stored tag on one entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRecord {
    pub key: String,
    pub value: Option<String>,
}

/// The entity a tag is attached to — its kind and id. `entity_id` is a string
/// because ids owned by other layers (users, teams) sit alongside this store's
/// uuids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRef {
    pub entity_type: String,
    pub entity_id: String,
}

/// An entity returned by a reverse (by-tag) lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedEntity {
    pub entity_type: String,
    pub entity_id: String,
}
