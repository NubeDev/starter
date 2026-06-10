//! Per-instance node registry: maps a config `type` name to the builder that
//! constructs the node from its JSON config.
//!
//! A plain instance with no static state — each runner and the flow manager hold
//! their own, so a test or a tenant can carry its own set of builders. The
//! builder names (`"memory"`, `"generate"`, `"sql"`, `"json_to_arrow"`, plus
//! nexus customs) match what stored flow configs already use, so a saved flow
//! parses and runs without migration.

use std::collections::HashMap;

use serde_json::Value;

use super::error::{EngineError, EngineResult};
use super::node::{Processor, Sink, Source};

/// Builds a [`Source`] from its JSON config (the node's `config` object).
pub type SourceBuilder = Box<dyn Fn(&Value) -> EngineResult<Box<dyn Source>> + Send + Sync>;

/// Builds a [`Processor`] from its JSON config.
pub type ProcessorBuilder = Box<dyn Fn(&Value) -> EngineResult<Box<dyn Processor>> + Send + Sync>;

/// Builds a [`Sink`] from its JSON config.
pub type SinkBuilder = Box<dyn Fn(&Value) -> EngineResult<Box<dyn Sink>> + Send + Sync>;

/// A set of node builders keyed by config `type` name. Construct one, register
/// the builders the deployment needs, then hand it to a [`super::pipeline`] to
/// turn a [`super::pipeline::PipelineConfig`] into runnable nodes.
#[derive(Default)]
pub struct Registry {
    sources: HashMap<String, SourceBuilder>,
    processors: HashMap<String, ProcessorBuilder>,
    sinks: HashMap<String, SinkBuilder>,
}

impl Registry {
    /// An empty registry. Register builders before building a pipeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a source builder under `name`. A later registration of the same
    /// name replaces the earlier one (last-wins), which lets an extension
    /// override a built-in without a separate de-register step.
    pub fn register_source(&mut self, name: impl Into<String>, builder: SourceBuilder) {
        self.sources.insert(name.into(), builder);
    }

    /// Register a processor builder under `name`. Last-wins, as [`register_source`].
    pub fn register_processor(&mut self, name: impl Into<String>, builder: ProcessorBuilder) {
        self.processors.insert(name.into(), builder);
    }

    /// Register a sink builder under `name`. Last-wins, as [`register_source`].
    pub fn register_sink(&mut self, name: impl Into<String>, builder: SinkBuilder) {
        self.sinks.insert(name.into(), builder);
    }

    /// Build a source named `name` from `config`, or [`EngineError::Build`] if no
    /// builder is registered under that name.
    pub fn build_source(&self, name: &str, config: &Value) -> EngineResult<Box<dyn Source>> {
        let builder = self
            .sources
            .get(name)
            .ok_or_else(|| unknown("source", name))?;
        builder(config)
    }

    /// Build a processor named `name` from `config`, or [`EngineError::Build`].
    pub fn build_processor(&self, name: &str, config: &Value) -> EngineResult<Box<dyn Processor>> {
        let builder = self
            .processors
            .get(name)
            .ok_or_else(|| unknown("processor", name))?;
        builder(config)
    }

    /// Build a sink named `name` from `config`, or [`EngineError::Build`].
    pub fn build_sink(&self, name: &str, config: &Value) -> EngineResult<Box<dyn Sink>> {
        let builder = self.sinks.get(name).ok_or_else(|| unknown("sink", name))?;
        builder(config)
    }
}

/// A build error that names the kind and the unknown type so the message points
/// straight at the offending node in a config.
fn unknown(kind: &str, name: &str) -> EngineError {
    EngineError::Build(format!("no {kind} builder registered for type {name:?}"))
}
