//! `flow-agent` — flow editor + AI agent chat demo.
//!
//! Library half is exposed so integration tests can call `server::build`
//! directly without spawning the binary.

pub mod domain;
pub mod flow_engine;
pub mod migrations;
pub mod rest;
pub mod server;
pub mod sse;
pub mod store;
