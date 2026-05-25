//! clickhouse goal — tool implementations.
//!
//! One verb per file — the rule.write / mart.create / retention.set
//! dispatch lives in the per-verb modules; the shared backing trait
//! and `Reversible` glue live in [`store`]. This barrel re-exports
//! the verb modules and contains no logic of its own. See
//! [docs/design/clickhouse-rules/](../../../docs/design/clickhouse-rules/README.md)
//! for the snapshot-before-write contract and the data-loss caveat
//! on mart.create undo.

pub mod mart_create;
pub mod mart_drop;
pub mod mart_list;
pub mod retention_set;
pub mod rule_list;
pub mod rule_write;
pub mod store;
pub mod tables_list;
