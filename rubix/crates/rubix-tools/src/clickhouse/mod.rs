//! clickhouse goal — tool implementations.
//!
//! One file per verb (FILE-LAYOUT §2). This barrel re-exports the
//! verb modules and contains no logic of its own.

pub mod rule_write;
pub mod mart_create;
pub mod retention_set;
