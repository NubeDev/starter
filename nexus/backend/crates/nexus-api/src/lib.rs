//! Nexus control-plane library surface.
//!
//! The binary (`main.rs`) and the OpenAPI generator (`bin/openapi.rs`) both
//! build on this. Transport handlers, middleware, app state, and the OpenAPI
//! aggregator live here so they can be exercised by integration tests without
//! going through `fn main`.

pub mod authz;
pub mod bootstrap;
pub mod identity;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod serve;
pub mod state;

pub use state::AppState;
