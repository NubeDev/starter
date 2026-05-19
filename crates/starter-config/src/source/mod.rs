//! Named source layers, in case a consumer wants to compose their
//! own layering policy outside the default [`super::loader::Loader`]
//! flow. For most consumers the loader is sufficient and this module
//! is unused.

mod kind;

pub use kind::SourceKind;
