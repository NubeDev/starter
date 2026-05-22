//! `flow-agent` — flow editor + AI agent chat demo.
//!
//! Library half is exposed so integration tests can call `server::build`
//! directly without spawning the binary.

pub mod agent_bridge;
pub mod ai_runtime;
pub mod builder_stream;
pub mod cache_demo;
pub mod domain;
pub mod flow_engine;
pub mod insights_mock;
pub mod migrations;
pub mod rest;
pub mod run_drain;
pub mod server;
pub mod sse;
pub mod store;
