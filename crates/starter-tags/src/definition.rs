//! `TagDefinition` — advisory dictionary (T5).
//!
//! The dictionary is **advisory**: writes are never refused on the
//! basis of a missing or wrong-typed definition. The storage backend
//! is hidden behind a small trait so this crate stays sync and
//! driver-free; `starter-store-postgres["dimensions"]` provides the
//! Postgres impl.

use serde::{Deserialize, Serialize};

/// Kind of a tag, as recorded in `tag_definitions`. T5 reconciles the
/// table column to four canonical values; there is deliberately **no
/// bare `Num`** — a numeric discriminant is `NumDiscriminant`, and the
/// underlying storage kind is still `Str`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    /// `TagValue::Bool` — sugar-friendly boolean.
    Bool,
    /// `TagValue::Str` — arbitrary string.
    Str,
    /// A ref-shaped string (`equip_*`, `site_*`, …). Truth lives in
    /// `entity_refs`; the tag is a denormalised optimisation.
    Ref,
    /// A `TagValue::Str` that holds a canonical decimal representation
    /// of an integer (port number, building id, firmware major). The
    /// storage kind is `Str` — *never* a JSON number — so equality is
    /// exact and the ClickHouse bloom-filter index works as designed.
    NumDiscriminant,
}

impl TagKind {
    /// Stable on-disk text representation. Matches the values written
    /// to the `tag_definitions.kind` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            TagKind::Bool => "bool",
            TagKind::Str => "str",
            TagKind::Ref => "ref",
            TagKind::NumDiscriminant => "num_discriminant",
        }
    }
}

/// One row of `tag_definitions`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TagDefinition {
    pub key: String,
    pub kind: TagKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional canonical value set, as a JSON array of strings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    /// When `kind == Ref`, the target entity kind (e.g. `"equip"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_kind: Option<String>,
    /// Provenance: `"builtin"`, `"pack:<id>"`, `"user"`, `"agent"`,
    /// or `"reserved"` for prefix-registry rows.
    pub source: String,
}

/// Storage trait for the dictionary. The Postgres impl lives in
/// `starter-store-postgres["dimensions"]` so this crate stays
/// driver-free (T1).
pub trait TagDictionary {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Fetch one definition by key.
    fn get(&self, key: &str) -> Result<Option<TagDefinition>, Self::Error>;

    /// Upsert a definition (no schema migration; advisory only).
    fn upsert(&self, def: &TagDefinition) -> Result<(), Self::Error>;

    /// List all known definitions. Used by autocomplete and the
    /// agent's `tag_entity` tool.
    fn list(&self) -> Result<Vec<TagDefinition>, Self::Error>;
}
