//! The native pipeline engine — the keystone that turns a streaming data engine
//! into a request/response and live-subscription product.
//!
//! A bounded-channel [`core::Pipeline`] runs `source → processors → sink` over
//! Arrow `RecordBatch`es via DataFusion directly. On top of it this crate adds
//! the product's two custom sinks (a bounded in-memory collector for one-shot
//! queries, a broadcast sink for live SSE), the source/processor/sink built-ins,
//! and the runners that drive a pipeline to completion or cancellation. The node
//! palette ([`describe`]) and the [`native_registry`] of builders are the two
//! halves of the flow-builder seam.

pub mod arrow_json;
pub mod core;
pub mod federation;
pub mod flow;
pub mod native_registry;
pub mod processor;
pub mod registry;
pub mod runner;
pub mod sink;
pub mod source;
pub mod stream_registry;
pub mod time;

pub use federation::{FederatedQuery, FederatedSource, PostgresConn};
pub use flow::{FlowManager, FlowMetrics, FlowStats, MetricsSnapshot};
pub use native_registry::native_registry;
pub use registry::{describe, NodeCategory, NodeDescriptor};
pub use runner::{LiveRunner, QueryOutcome, QueryRunner};
pub use sink::cap::Caps;
pub use stream_registry::{attach, register, Attach, StreamKey, Subscription};
