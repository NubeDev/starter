//! The ArkFlow bridge — the keystone that turns a streaming engine into a
//! request/response and live-subscription product.
//!
//! ArkFlow owns connectors, DataFusion SQL, and Arrow streaming but has no way
//! to hand rows back to a caller; its only outputs go to stdout/Kafka/etc. This
//! crate adds the two custom output sinks (a bounded in-memory collector for
//! one-shot queries, and a broadcast sink for live SSE) and the runners that
//! drive a `Stream` to completion or cancellation. ArkFlow is a pinned git
//! dependency, never forked.

pub mod arrow_json;
pub mod registry;
pub mod runner;
pub mod sink;
pub mod stream_registry;

pub use registry::register_all;
pub use runner::{LiveRunner, QueryOutcome, QueryRunner};
pub use sink::cap::Caps;
pub use stream_registry::{attach, register, Attach, StreamKey, Subscription};
