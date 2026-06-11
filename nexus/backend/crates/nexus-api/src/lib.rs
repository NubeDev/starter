//! Nexus control-plane library surface.
//!
//! The binary (`main.rs`) and the OpenAPI generator (`bin/openapi.rs`) both
//! build on this. Transport handlers, middleware, app state, and the OpenAPI
//! aggregator live here so they can be exercised by integration tests without
//! going through `fn main`.

pub mod agents;
pub mod alerting;
pub mod authz;
pub mod boot;
pub mod bootstrap;
pub mod cache;
pub mod changelog;
pub mod datasource_kinds;
pub mod datasource_pools;
pub mod detecting;
pub mod extensions;
pub mod federation;
pub mod flows;
pub mod identity;
pub mod ingest;
pub mod insights;
pub mod kinds;
pub mod middleware;
pub mod openapi;
pub mod prefs;
pub mod quota;
pub mod ratelimit;
pub mod reversible;
pub mod routes;
pub mod serve;
pub mod state;

pub use state::AppState;
