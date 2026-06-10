//! Native pipeline processors over the RW-01 [`crate::core::Processor`] trait.
//!
//! The two transforms a light-ingestion flow needs: `json_to_arrow` parses a
//! JSON carrier batch into a typed Arrow batch, and `sql` runs a DataFusion
//! statement over the in-flight batch. Both keep the config shapes and registry
//! names stored flow configs already use, so a saved flow runs unchanged.

pub mod declared_schema;
pub mod json_to_arrow;
pub mod sql;

pub use json_to_arrow::JsonToArrow;
pub use sql::SqlProcessor;
