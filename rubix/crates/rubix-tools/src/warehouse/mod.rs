//! Rubix warehouse verbs — TimescaleDB-backed.
//!
//! The previous ClickHouse-backed verbs were deleted in stage 3 of
//! `rubix/docs/proposal/warehouse-engine-swap.md`. This module
//! rebuilds them against the TimescaleDB engine, reading and
//! writing through [`starter_store_warehouse::WarehouseClient`]:
//!
//! - `ingest`          — append synth meter readings into `samples`.
//! - `rule.list`       — enumerate continuous aggregates tagged as
//!                       rules (derived-state views).
//! - `mart.list`       — enumerate continuous aggregates tagged as
//!                       marts (history / aggregate views).
//! - `tables.list`     — enumerate hypertables with engine + retention.
//! - `rule.write`      — execute CREATE/ALTER DDL for a rule; returns
//!                       prior view definition in the response.
//! - `mart.create`     — provision a new mart; idempotent.
//! - `mart.drop`       — drop a mart; idempotent.
//! - `retention.set`   — add / remove a retention policy.
//!
//! Rule-vs-mart distinction is naming-convention only: caggs whose
//! name starts with `rule_` or ends with `_rule` surface via
//! `rule.list`; everything else surfaces via `mart.list`. A future
//! registration table can replace this probe without changing the
//! DTO shape.

pub mod ingest;
pub mod mart_create;
pub mod mart_drop;
pub mod mart_list;
pub mod retention_set;
pub mod rule_list;
pub mod rule_write;
pub mod tables_list;
