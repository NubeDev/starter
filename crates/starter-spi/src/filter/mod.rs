//! Filter primitives. The starter wire surface intentionally keeps
//! filtering simple — equality and `IN` only. Consumers needing
//! richer query languages (RSQL, GraphQL) layer them in their own
//! crate.

#[allow(clippy::module_inception)]
mod filter;
mod predicate;

pub use filter::Filter;
pub use predicate::Predicate;
