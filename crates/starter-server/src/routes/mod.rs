//! Starter-owned routes. Each route is one file so an AI loading
//! "the health route" gets exactly that and nothing else.

mod health;
mod metrics;
mod openapi_doc;

pub use health::health_router;
pub use metrics::metrics_router;
pub use openapi_doc::openapi_router;
