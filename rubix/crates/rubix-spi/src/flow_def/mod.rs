//! Flow-definition SPI — async store trait + value types for the
//! `flow_ops.deploy` / `flow_ops.list` / `flow_ops.duplicate`
//! family of rubix verbs.
//!
//! Mirrors [`crate::dashboard`] in shape: the trait + wire shapes
//! live here in `rubix-spi` (no `sqlx`, no HTTP, no runtime), the
//! Postgres impl lives in [`rubix-store-postgres::flows`], and the
//! in-memory test impl + the `Reversible` glue stay alongside the
//! verb bodies in [`rubix-tools::flow_ops::store`].
//!
//! See [`rubix/docs/sessions/2026-05-25-tick-counter-r3-and-flow-ops-pg.md`]
//! for the rationale of the split (the trait used to be in
//! rubix-tools, which prevented rubix-store-postgres from
//! depending on it).

pub mod store;

pub use store::{FlowDefChange, FlowDefStore, FlowRevisionRow, FLOW_DEFINITION_KIND};
