//! HTTP-middleware data types. The transport-side factories that turn
//! these into `tower::Layer` impls live in `starter-server` so the
//! concrete `http::Request` body type is pinned in one place.

mod latency;
mod request_id;

pub use request_id::{RequestId, HEADER_NAME as REQUEST_ID_HEADER};
