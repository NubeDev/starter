//! Nexus control-plane library surface.
//!
//! The binary (`main.rs`) and the OpenAPI generator (`bin/openapi.rs`) both
//! build on this. Transport handlers, middleware, app state, and the OpenAPI
//! aggregator live here so they can be exercised by integration tests without
//! going through `fn main`.

pub mod agents;
pub mod alerting;
pub mod authz;
pub mod bootstrap;
pub mod datasource_pools;
pub mod identity;
pub mod kinds;
pub mod middleware;
pub mod openapi;
pub mod prefs;
pub mod routes;
pub mod serve;
pub mod state;

pub use state::AppState;
