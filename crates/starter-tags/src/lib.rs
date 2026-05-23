//! `starter-tags` — the workspace's shared tag vocabulary.
//!
//! See `DOCS/Tags/SCOPE.md`. This crate is deliberately tiny, sync,
//! and driver-free: it ships the types ([`TagSet`], [`TagValue`],
//! [`TagQuery`], [`TagDefinition`]) and the three compilation targets
//! (Postgres [`compile_to_pg`], ClickHouse [`compile_to_ch`],
//! in-process [`compile_to_match`]). Every consumer crate that filters
//! or routes by tags depends on this one.

pub mod compile_ch;
pub mod compile_match;
pub mod compile_pg;
pub mod definition;
pub mod error;
pub mod query;
pub mod reserved;
pub mod set;

pub use compile_ch::{compile_to_ch, ChCompileOptions};
pub use compile_match::{compile_to_match, matches};
pub use compile_pg::{compile_to_pg, PgCompileOptions, SqlFragment};
pub use definition::{TagDefinition, TagDictionary, TagKind};
pub use error::{TagParseError, TagSetError};
pub use query::TagQuery;
pub use reserved::{is_reserved, ReservedKey, RESERVED_KEYS};
pub use set::{tag_value_to_ch_string, TagSet, TagValue};

/// Public prelude — `use starter_tags::prelude::*;`.
pub mod prelude {
    pub use crate::compile_ch::{compile_to_ch, ChCompileOptions};
    pub use crate::compile_match::{compile_to_match, matches};
    pub use crate::compile_pg::{compile_to_pg, PgCompileOptions, SqlFragment};
    pub use crate::definition::{TagDefinition, TagDictionary, TagKind};
    pub use crate::error::{TagParseError, TagSetError};
    pub use crate::query::TagQuery;
    pub use crate::set::{tag_value_to_ch_string, TagSet, TagValue};
}
