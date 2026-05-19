//! Typed identifiers. `Id<T>` carries the resource type at the type
//! level so an `Id<User>` cannot be passed where an `Id<Project>` is
//! expected.

mod typed;

pub use typed::Id;
