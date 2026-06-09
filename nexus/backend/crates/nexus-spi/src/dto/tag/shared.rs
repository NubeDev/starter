//! Tag wire types, shared across the tag verbs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The kinds of entity that can be tagged. The set the API accepts — a tag's
/// target is referenced by kind + id, so a kind owned by another layer (a user,
/// a team) is taggable without this crate owning the entity. New variants are
/// add-only within a major.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaggableKind {
    Dashboard,
    Datasource,
    Flow,
    AlertRule,
    User,
    Team,
}

/// One tag: a `key` with an optional `value`. `value` absent (or null) is a
/// bare label; present is a key:value pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Tag {
    /// The tag key, e.g. `temp` or `building`.
    pub key: String,
    /// The tag value, e.g. `abc`. Absent for a bare label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Body for `PUT …/tags` — the complete tag set to persist on an entity. This
/// is a full replace: tags not listed are removed, so the client sends the
/// whole set it wants, not a delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SetTagsRequest {
    pub tags: Vec<Tag>,
}

/// An entity returned by a by-tag lookup: its kind and id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TaggedEntity {
    pub kind: TaggableKind,
    /// The entity's id as a string (ids owned by other layers are not uuids).
    pub id: String,
}
