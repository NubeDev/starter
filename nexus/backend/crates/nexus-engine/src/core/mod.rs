//! Native pipeline engine core — zero ArkFlow imports.
//!
//! The self-contained replacement for ArkFlow's `StreamConfig → Stream →
//! run(token)` loop and builder registry: node traits over Arrow `RecordBatch`,
//! a per-instance builder registry, a JSON config parser, and a bounded-channel
//! pipeline with cooperative cancellation. RW-02 ports the real nodes onto these
//! traits; RW-03 moves the runners off ArkFlow and onto [`pipeline::Pipeline`].

mod config;
mod error;
mod node;
mod outcome;
mod pipeline;
mod registry;

pub use config::{NodeSpec, PipelineConfig, DEFAULT_BUFFER_CAPACITY};
pub use error::{EngineError, EngineResult};
pub use node::{Processor, Sink, Source};
pub use outcome::RunOutcome;
pub use pipeline::Pipeline;
pub use registry::{ProcessorBuilder, Registry, SinkBuilder, SourceBuilder};
