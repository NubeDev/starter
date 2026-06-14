//! Native pipeline engine core — node traits over Arrow `RecordBatch`, a
//! per-instance builder registry, a JSON config parser, and a bounded-channel
//! pipeline with cooperative cancellation.
//!
//! The whole engine — the `source → channel → processors → sink` loop and the
//! builder registry — is self-contained here, calling DataFusion / arrow-json
//! directly. The runners ([`crate::runner`]) and the flow manager drive a
//! [`pipeline::Pipeline`]; the built-in nodes register against [`registry::Registry`].

mod config;
mod error;
mod node;
mod outcome;
mod pipeline;
mod registry;
mod slice;

pub use config::{NodeSpec, PipelineConfig, DEFAULT_BUFFER_CAPACITY, DEFAULT_MAX_BATCH_ROWS};
pub use error::{EngineError, EngineResult};
pub use node::{Processor, Sink, Source};
pub use outcome::RunOutcome;
pub use pipeline::Pipeline;
pub use registry::{ProcessorBuilder, Registry, SinkBuilder, SourceBuilder};
