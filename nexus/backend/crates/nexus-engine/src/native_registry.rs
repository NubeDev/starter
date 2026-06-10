//! Build a [`core::Registry`] populated with every native node builder.
//!
//! The factory that wires the RW-02 source/processor/sink ports into the RW-01
//! registry under the same string names the stored flow configs already use
//! (`"memory"`, `"generate"`, `"http_poll"`, `"simulator"`, `"json_to_arrow"`,
//! `"sql"`, `"collector"`, `"sse"`, `"postgres"`, `"drop"`, `"stdout"`). A flow
//! built against this registry runs the native pipeline with no config change.
//! RW-03 hands this registry to the runners in place of the ArkFlow stream
//! build; until then it lives alongside the ArkFlow registrations.

use crate::core::Registry;
use crate::processor::{JsonToArrow, SqlProcessor};
use crate::sink::{CollectorSink, DropSink, PostgresSink, SseSink, StdoutSink};
use crate::source::{GenerateSource, HttpPollSource, MemorySource, SimulatorSource};

/// Construct a registry holding every native built-in. Last-wins registration
/// means a later override (e.g. an extension) can replace a built-in by name.
pub fn native_registry() -> Registry {
    let mut registry = Registry::new();

    registry.register_source(
        "memory",
        Box::new(|c| Ok(Box::new(MemorySource::from_config(c)?))),
    );
    registry.register_source(
        "generate",
        Box::new(|c| Ok(Box::new(GenerateSource::from_config(c)?))),
    );
    registry.register_source(
        "http_poll",
        Box::new(|c| Ok(Box::new(HttpPollSource::from_config(c)?))),
    );
    registry.register_source(
        "simulator",
        Box::new(|c| Ok(Box::new(SimulatorSource::from_config(c)?))),
    );

    registry.register_processor(
        "json_to_arrow",
        Box::new(|c| Ok(Box::new(JsonToArrow::from_config(c)?))),
    );
    registry.register_processor(
        "sql",
        Box::new(|c| Ok(Box::new(SqlProcessor::from_config(c)?))),
    );

    registry.register_sink(
        "collector",
        Box::new(|c| Ok(Box::new(CollectorSink::from_config(c)?))),
    );
    registry.register_sink("sse", Box::new(|c| Ok(Box::new(SseSink::from_config(c)?))));
    registry.register_sink(
        "postgres",
        Box::new(|c| Ok(Box::new(PostgresSink::from_config(c)?))),
    );
    registry.register_sink("drop", Box::new(|_| Ok(Box::new(DropSink::new()))));
    registry.register_sink("stdout", Box::new(|_| Ok(Box::new(StdoutSink::new()))));

    registry
}
