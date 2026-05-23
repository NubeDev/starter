//! Rubix HTTP client.
//!
//! Thin extension of `starter-client-rs` adding the rubix endpoint
//! surface. Layout mirrors [`rubix-tools`] one-to-one: one subfolder
//! per goal, one file per verb.

pub mod analytics;
pub mod clickhouse;
pub mod clipboard;
pub mod dashboard;
pub mod flow_ops;
pub mod system;
pub mod tags;
pub mod team;
pub mod tenant;
pub mod undo;
pub mod user;
