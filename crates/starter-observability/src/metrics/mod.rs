//! Prometheus registry + the small set of metrics every
//! starter-server emits. Consumers register their own metrics on the
//! returned registry; they don't have to create a second one.

mod registry;
mod standard;

pub use registry::registry;
pub use standard::StandardMetrics;
