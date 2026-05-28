//! `/api/v1/admin/*` wire types.
//!
//! Every list endpoint under the admin surface returns the same
//! envelope: a [`Page<RegistryItem>`](starter_spi::paging::Page).
//! Per-kind extras ride inside [`RegistryItem::metadata`] so new
//! keys are additive without a wire bump. See
//! [docs/design/admin/](../../../../docs/design/admin/README.md).

pub mod item;
pub mod kind;
pub mod overview;
pub mod snapshot;
pub mod source;

pub use item::RegistryItem;
pub use kind::RegistryKind;
pub use overview::RegistryOverview;
pub use snapshot::RegistrySnapshot;
pub use source::ItemSource;
