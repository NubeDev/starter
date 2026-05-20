//! `starter-notes` — a demo consumer of the starter library set. The
//! library half is exposed so the integration test in `tests/` can
//! call `server::build` directly without spawning the binary.

pub mod cli;
pub mod domain;
pub mod flow_demo;
pub mod grpc;
pub mod mcp;
pub mod migrations;
pub mod rest;
pub mod server;
