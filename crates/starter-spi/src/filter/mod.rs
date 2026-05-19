//! Filter primitives. The starter wire surface intentionally keeps
//! filtering simple — equality and `IN` only. Consumers needing
//! richer query languages (RSQL, GraphQL) layer them in their own
//! crate.

mod predicate;

pub use predicate::Predicate;
