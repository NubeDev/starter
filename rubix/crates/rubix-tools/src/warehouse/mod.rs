//! warehouse goal — persistence tools (L1+) and warehouse-ruler verbs.
//!
//! See `rubix/docs/sessions/data-flow/02-ingest-l1.md` for the
//! framework: persistence is a tool, delivery is a flow. The
//! `rule.write` / `mart.create` / `retention.set` verbs (formerly
//! `clickhouse.*`) live alongside the data-flow tools here as one
//! vendor-neutral warehouse surface; see
//! [docs/design/warehouse-rules/](../../../docs/design/warehouse-rules/README.md)
//! for the snapshot-before-write contract.

pub mod anomaly_gate;
pub mod clean_minute;
pub mod ingest;
pub mod mart_create;
pub mod mart_drop;
pub mod mart_list;
pub mod retention_set;
pub mod rollup_15m;
pub mod rule_list;
pub mod rule_write;
pub mod store;
pub mod tables_list;
pub mod warehouse_client_writer;
