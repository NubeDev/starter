//! axum-flavored middleware for starter-server. Data types
//! (`RequestId`, `StandardMetrics`) live in `starter-observability`;
//! the helpers here mount them onto an `axum::Router`.

mod latency;
mod request_id;

pub use latency::with_latency;
pub use request_id::with_request_id;
