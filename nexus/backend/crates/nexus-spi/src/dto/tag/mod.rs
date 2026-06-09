//! Tag DTOs — a tenant-scoped label (`key` + optional `value`) attached to any
//! entity. A bare label like `temp` is a key with no value; `building=abc` is a
//! key with a value. The same shape covers both.

mod shared;

pub use shared::{SetTagsRequest, Tag, TaggableKind, TaggedEntity};
