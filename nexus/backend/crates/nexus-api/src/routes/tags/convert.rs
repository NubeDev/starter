//! Map between the tag wire DTOs and the store's tag records.
//!
//! The one place the `TaggableKind` ⇄ stored `entity_type` string mapping lives,
//! so every tag route agrees on the persisted kind names. A new taggable kind is
//! one arm here plus the `nexus-spi` enum variant.

use nexus_spi::dto::tag::{Tag, TaggableKind, TaggedEntity};
use nexus_store::tag::{TagRecord, TaggedEntity as StoredTaggedEntity};

/// The stored `entity_type` string for a wire kind.
pub fn kind_to_stored(kind: TaggableKind) -> &'static str {
    match kind {
        TaggableKind::Dashboard => "dashboard",
        TaggableKind::Datasource => "datasource",
        TaggableKind::Flow => "flow",
        TaggableKind::Detection => "detection",
        TaggableKind::User => "user",
        TaggableKind::Team => "team",
    }
}

/// The wire kind for a stored `entity_type` string. An unrecognized value
/// (written by a newer server) maps to `None` so a reverse lookup can skip it
/// rather than fail the read.
pub fn kind_of(stored: &str) -> Option<TaggableKind> {
    Some(match stored {
        "dashboard" => TaggableKind::Dashboard,
        "datasource" => TaggableKind::Datasource,
        "flow" => TaggableKind::Flow,
        "detection" => TaggableKind::Detection,
        "user" => TaggableKind::User,
        "team" => TaggableKind::Team,
        _ => return None,
    })
}

/// Wire tag → store record.
pub fn to_record(tag: &Tag) -> TagRecord {
    TagRecord {
        key: tag.key.clone(),
        value: tag.value.clone(),
    }
}

/// Store record → wire tag.
pub fn to_dto(rec: &TagRecord) -> Tag {
    Tag {
        key: rec.key.clone(),
        value: rec.value.clone(),
    }
}

/// Store tagged-entity → wire, dropping any whose stored kind this binary does
/// not recognize.
pub fn to_tagged_entity(rec: &StoredTaggedEntity) -> Option<TaggedEntity> {
    Some(TaggedEntity {
        kind: kind_of(&rec.entity_type)?,
        id: rec.entity_id.clone(),
    })
}
