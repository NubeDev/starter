//! Parse the pipeline JSON shape into typed node specs.
//!
//! The shape mirrors what the runners build today (`runner/query.rs`,
//! `runner/live.rs`, `flow/manager.rs`): `{ "input": <node>, "pipeline": {
//! "processors": [<node>…], "buffer_capacity"? }, "output": <node> }`. Each
//! `<node>` is an object with a `"type"` string naming a registry builder; the
//! whole node object (including `type`) is passed to the builder as its config,
//! matching how ArkFlow nodes read their own `type` siblings.

use serde::Deserialize;
use serde_json::Value;

use super::error::{EngineError, EngineResult};

/// Default bounded-channel capacity between source and sink when the config does
/// not set `pipeline.buffer_capacity`. Small enough to exert backpressure on a
/// fast source, large enough to amortise per-batch handoff cost.
pub const DEFAULT_BUFFER_CAPACITY: usize = 64;

/// One node: its builder `type` name and the full config object handed to the
/// builder. The config retains the `type` key so a builder may read siblings
/// without re-plumbing.
#[derive(Debug, Clone)]
pub struct NodeSpec {
    /// The registry key naming the builder.
    pub node_type: String,
    /// The node's full config object (including `type`).
    pub config: Value,
}

/// A parsed pipeline: one source, an ordered processor chain, one sink, and the
/// source→sink channel capacity.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// The input node spec.
    pub input: NodeSpec,
    /// The processor chain, applied in order.
    pub processors: Vec<NodeSpec>,
    /// The output node spec.
    pub output: NodeSpec,
    /// Bounded channel capacity between the source task and the sink loop.
    pub buffer_capacity: usize,
}

/// The raw JSON envelope before node specs are extracted.
#[derive(Deserialize)]
struct RawConfig {
    input: Value,
    #[serde(default)]
    pipeline: RawPipeline,
    output: Value,
}

#[derive(Deserialize, Default)]
struct RawPipeline {
    #[serde(default)]
    processors: Vec<Value>,
    /// Optional override of [`DEFAULT_BUFFER_CAPACITY`].
    buffer_capacity: Option<usize>,
}

impl PipelineConfig {
    /// Parse the pipeline envelope. Returns [`EngineError::Build`] if the shape
    /// is malformed or any node is missing its `type`.
    pub fn from_value(value: Value) -> EngineResult<Self> {
        let raw: RawConfig = serde_json::from_value(value)
            .map_err(|e| EngineError::Build(format!("invalid pipeline config: {e}")))?;

        let input = node_spec(raw.input, "input")?;
        let output = node_spec(raw.output, "output")?;
        let processors = raw
            .pipeline
            .processors
            .into_iter()
            .map(|p| node_spec(p, "processor"))
            .collect::<EngineResult<Vec<_>>>()?;
        let buffer_capacity = raw
            .pipeline
            .buffer_capacity
            .unwrap_or(DEFAULT_BUFFER_CAPACITY)
            .max(1);

        Ok(Self {
            input,
            processors,
            output,
            buffer_capacity,
        })
    }
}

/// Extract a [`NodeSpec`] from a node JSON object, requiring a string `type`.
/// `role` ("input"/"output"/"processor") sharpens the error message.
fn node_spec(value: Value, role: &str) -> EngineResult<NodeSpec> {
    let node_type = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| EngineError::Build(format!("{role} node missing string \"type\"")))?
        .to_string();
    Ok(NodeSpec {
        node_type,
        config: value,
    })
}
