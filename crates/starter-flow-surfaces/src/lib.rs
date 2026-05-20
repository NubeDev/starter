//! # starter-flow-surfaces
//!
//! Flow ↔ Tool / Service wrappers per `DOCS/flow/scope/SCOPE.md` R8
//! and R9:
//!
//! - [`FlowAsTool`] — wraps a flow as `starter_spi::tool::Tool`. Makes
//!   every flow automatically MCP-callable, REST-callable,
//!   CLI-callable, and callable from another flow as a `tool-call`
//!   node.
//! - [`FlowAsService`] — wraps a flow as
//!   `starter_spi::service::Service`. Reads from an `EventSink`;
//!   invokes the flow per event. Body lands in stage 8.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::timeout;

use starter_flow::engine::Engine;
use starter_flow::propagator::FlowTopology;
use starter_flow::run::{
    FlowRunner, FlowRunnerConfig, InMemoryRunStore, RunCancel, RunSpec, RunStore as FlowRunStore,
};
use starter_flow::state::RunStatus;
use starter_flow_spi::flow::{FlowEvent, FlowId, FlowRevisionId};
use starter_flow_spi::node::{KindId, SlotMap, SlotRef, SlotValue};
use starter_flow_spi::Cancel;
use starter_spi::error::{Error as SpiError, Result as SpiResult};
use starter_spi::tool::{Tool, ToolDefinition};

// ---------------------------------------------------------------------------
// FlowAsTool — SCOPE R8 + D-F3.4.
// ---------------------------------------------------------------------------

/// Adapter mapping a [`Tool::invoke`] JSON input into the seed
/// slot writes the wrapped flow's propagator consumes.
///
/// D-F3.4 pins schemas explicitly (no derive-from-flow-revision);
/// the seed adapter is the matching imperative side — explicit
/// and per-flow.
pub type SeedAdapter =
    Arc<dyn Fn(&serde_json::Value) -> Vec<(SlotRef, SlotValue)> + Send + Sync + 'static>;

/// Adapter mapping the flow's terminal-slot read-back into the
/// JSON value [`Tool::invoke`] returns to the caller.
pub type OutputAdapter = Arc<dyn Fn(&SlotMap) -> serde_json::Value + Send + Sync + 'static>;

/// Wraps a flow as a `starter_spi::tool::Tool` (SCOPE R8 + D-F3.4).
///
/// Per "Nodes are not Tools — Tools are one node kind": the
/// outside world sees a single first-class `Tool`; internally the
/// flow's `tool-call` nodes remain the only place where
/// `Tool::call` actually fires. Wrapping a flow this way makes
/// it MCP-callable, REST-callable, CLI-callable, and callable
/// from another flow as a `tool-call` node — for free.
///
/// `Tool::invoke` constructs a one-shot [`FlowRunner`] off the
/// supplied [`Engine`] (pulling its shared `HealthHandle` and any
/// attached SPI `RunStore` so stage-6 durability semantics apply
/// transparently), drives the flow once, and reads back the
/// terminal output via the [`OutputAdapter`].
///
/// Construction takes everything explicitly:
///
/// - `tool_id` validates as a [`KindId`] (R10 reverse-DNS).
/// - `input_schema` / `output_schema` are not derived from the
///   flow revision (D-F3.4); callers supply them.
/// - `topology` and `terminal_slots` come from the caller because
///   the Phase-3 `FlowRegistry::resolve` path that derives them
///   from `(flow_id, revision)` lands later; once it does, the
///   convenience constructor here can pull from the engine's
///   `flows` / `node_kinds` registries.
///
/// Cancellation: the SPI [`Tool`] trait's `invoke` carries no
/// `Cancel`, so the standard surface drives the flow to
/// completion. Hosts needing cancellation (timeouts, client
/// disconnect) call [`FlowAsTool::invoke_with_cancel`] or
/// [`FlowAsTool::invoke_with_timeout`]; both forward into the
/// per-run cancel within a few hundred milliseconds (R13).
pub struct FlowAsTool {
    flow_id: FlowId,
    revision: FlowRevisionId,
    topology: Arc<FlowTopology>,
    terminal_slots: Vec<SlotRef>,
    engine: Arc<Engine>,
    tool_id: KindId,
    name: String,
    description: String,
    input_schema: serde_json::Value,
    output_schema: serde_json::Value,
    seed_adapter: SeedAdapter,
    output_adapter: OutputAdapter,
}

impl FlowAsTool {
    /// Start a fresh [`FlowAsToolBuilder`].
    pub fn builder() -> FlowAsToolBuilder {
        FlowAsToolBuilder::default()
    }

    /// Borrow the [`FlowId`] this wrapper invokes.
    pub fn flow_id(&self) -> &FlowId {
        &self.flow_id
    }

    /// Borrow the reverse-DNS [`KindId`] this tool is registered
    /// under (R10).
    pub fn tool_id(&self) -> &KindId {
        &self.tool_id
    }

    /// Borrow the explicit output schema (D-F3.4).
    pub fn output_schema(&self) -> &serde_json::Value {
        &self.output_schema
    }

    /// Invoke the flow with an explicit cancel handle. The standard
    /// [`Tool::invoke`] path delegates to this with a fresh,
    /// never-cancelled handle.
    ///
    /// R13: a fired `cancel` flips the run's per-run [`RunCancel`]
    /// within a few hundred milliseconds; the run reports
    /// `Cancelled` and this function returns
    /// [`SpiError::Internal`] wrapping a `flow run cancelled`
    /// source so the caller can distinguish abort-from-cancel from
    /// abort-from-node-failure.
    pub async fn invoke_with_cancel(
        &self,
        input: serde_json::Value,
        cancel: Arc<RunCancel>,
    ) -> SpiResult<serde_json::Value> {
        let span = tracing::info_span!(
            "flow_as_tool.call",
            flow_id = %self.flow_id,
            tool_id = %self.tool_id.as_str(),
        );
        let _enter = span.enter();

        let seeds = (self.seed_adapter)(&input);
        let spec = RunSpec::new(
            self.flow_id.clone(),
            self.revision,
            self.topology.clone(),
            seeds,
            self.terminal_slots.clone(),
        );

        // Construct a one-shot runner off the engine. The runner
        // shares the engine's `HealthHandle` (stage 6 D-F3.11) so
        // a degraded engine rejects this invocation early; if an
        // SPI `RunStore` is attached on the engine, the per-tick
        // checkpoint / retry-with-backoff / Degraded-mode plumbing
        // applies transparently.
        let mut runner = FlowRunner::new(
            self.engine.store.clone(),
            Arc::new(InMemoryRunStore::new()) as Arc<dyn FlowRunStore>,
        )
        .with_health_handle(self.engine.health_handle())
        .with_config(FlowRunnerConfig::default());
        if let Some(spi_store) = self.engine.run_store() {
            runner = runner.with_spi_run_store(spi_store.clone());
        }

        let mut handle =
            runner
                .start(spec, SlotMap::new())
                .await
                .map_err(|e| SpiError::Internal {
                    source: Box::new(std::io::Error::other(format!("flow start refused: {e}"))),
                })?;

        // Forward the caller's cancel into the per-run handle.
        // Drop on early return / drop-of-future is fine: the
        // forwarder task aborts itself once the run terminates.
        let run_cancel = handle.cancel.clone();
        let forwarder_cancel = cancel.clone();
        let forwarder = tokio::spawn(async move {
            forwarder_cancel.cancelled().await;
            run_cancel.cancel();
        });

        // Watch the event stream for `NodeFailed` so a per-node
        // error from a node-kind body surfaces as a typed
        // [`SpiError`] even though the engine's quiescence-based
        // termination reports the run as `Completed` (a single
        // failing node does not by itself flip the engine's
        // RunStatus to Failed — only propagator-level errors do).
        let node_failure: Arc<Mutex<Option<(String, String)>>> = Arc::new(Mutex::new(None));
        let mut events_rx = handle.events_tx.subscribe();
        let node_failure_for_task = node_failure.clone();
        let watcher = tokio::spawn(async move {
            while let Ok(ev) = events_rx.recv().await {
                if let FlowEvent::NodeFailed { node, error, .. } = ev {
                    let mut slot = node_failure_for_task
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    if slot.is_none() {
                        *slot = Some((node.to_string(), error));
                    }
                }
            }
        });

        let status = match (&mut handle.join).await {
            Ok(status) => status,
            Err(join_err) => {
                forwarder.abort();
                watcher.abort();
                return Err(SpiError::Internal {
                    source: Box::new(join_err),
                });
            }
        };
        forwarder.abort();
        watcher.abort();

        match status {
            RunStatus::Completed => {
                if let Some((node, error)) = node_failure
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
                {
                    return Err(SpiError::Internal {
                        source: Box::new(std::io::Error::other(format!(
                            "flow run failed: node {node} returned {error}"
                        ))),
                    });
                }
                let mut output: SlotMap = SlotMap::new();
                for sr in &self.terminal_slots {
                    if let Ok(v) = self.engine.store.read_slot(sr).await {
                        output.insert(format!("{}.{}", sr.node, sr.slot), v);
                    }
                }
                Ok((self.output_adapter)(&output))
            }
            RunStatus::Failed(error) => Err(SpiError::Internal {
                source: Box::new(std::io::Error::other(format!("flow run failed: {error}"))),
            }),
            RunStatus::Cancelled => Err(SpiError::Internal {
                source: Box::new(std::io::Error::other("flow run cancelled")),
            }),
            other => Err(SpiError::Internal {
                source: Box::new(std::io::Error::other(format!(
                    "flow run terminated in non-terminal status: {other:?}"
                ))),
            }),
        }
    }

    /// Convenience wrapper that bounds total invocation time. On
    /// timeout the per-run cancel is fired, the propagator drains,
    /// and the function returns [`SpiError::Internal`] wrapping a
    /// `flow invocation timed out` source.
    pub async fn invoke_with_timeout(
        &self,
        input: serde_json::Value,
        deadline: Duration,
    ) -> SpiResult<serde_json::Value> {
        let cancel = RunCancel::new();
        match timeout(deadline, self.invoke_with_cancel(input, cancel.clone())).await {
            Ok(res) => res,
            Err(_) => {
                cancel.cancel();
                Err(SpiError::Internal {
                    source: Box::new(std::io::Error::other("flow invocation timed out")),
                })
            }
        }
    }
}

#[async_trait]
impl Tool for FlowAsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
        }
    }

    async fn invoke(&self, input: serde_json::Value) -> SpiResult<serde_json::Value> {
        self.invoke_with_cancel(input, RunCancel::new()).await
    }
}

// ---------------------------------------------------------------------------
// FlowAsToolBuilder
// ---------------------------------------------------------------------------

/// Builder for [`FlowAsTool`]. Every field is required; missing
/// fields surface as [`FlowAsToolBuildError::MissingField`] from
/// [`FlowAsToolBuilder::build`].
#[must_use = "FlowAsToolBuilder does nothing until `.build()` is called"]
#[derive(Default)]
pub struct FlowAsToolBuilder {
    flow_id: Option<FlowId>,
    revision: Option<FlowRevisionId>,
    topology: Option<Arc<FlowTopology>>,
    terminal_slots: Vec<SlotRef>,
    engine: Option<Arc<Engine>>,
    tool_id: Option<KindId>,
    name: Option<String>,
    description: Option<String>,
    input_schema: Option<serde_json::Value>,
    output_schema: Option<serde_json::Value>,
    seed_adapter: Option<SeedAdapter>,
    output_adapter: Option<OutputAdapter>,
}

/// Error from [`FlowAsToolBuilder::build`].
#[derive(Debug)]
#[non_exhaustive]
pub enum FlowAsToolBuildError {
    /// One or more required fields were not set; the value names
    /// the first missing field encountered.
    MissingField(&'static str),
}

impl std::fmt::Display for FlowAsToolBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(name) => write!(f, "FlowAsTool builder missing field: {name}"),
        }
    }
}

impl std::error::Error for FlowAsToolBuildError {}

impl FlowAsToolBuilder {
    /// Required: the flow this tool wraps.
    pub fn flow_id(mut self, id: FlowId) -> Self {
        self.flow_id = Some(id);
        self
    }
    /// Required: the immutable revision the wrapper pins.
    pub fn revision(mut self, rev: FlowRevisionId) -> Self {
        self.revision = Some(rev);
        self
    }
    /// Required: the propagator topology the flow runs on.
    pub fn topology(mut self, topology: Arc<FlowTopology>) -> Self {
        self.topology = Some(topology);
        self
    }
    /// Required (non-empty): the terminal slots the wrapper reads
    /// back at the end of a successful run.
    pub fn terminal_slots(mut self, slots: Vec<SlotRef>) -> Self {
        self.terminal_slots = slots;
        self
    }
    /// Required: the engine handle. The wrapper pulls `store`,
    /// `health_handle`, and `run_store` from here.
    pub fn engine(mut self, engine: Arc<Engine>) -> Self {
        self.engine = Some(engine);
        self
    }
    /// Required: the reverse-DNS tool id (R10).
    pub fn tool_id(mut self, id: KindId) -> Self {
        self.tool_id = Some(id);
        self
    }
    /// Required: the tool's `name` (transported in
    /// [`ToolDefinition`]).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    /// Required: the one-sentence human description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    /// Required: explicit input JSON-schema (D-F3.4).
    pub fn input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }
    /// Required: explicit output JSON-schema (D-F3.4).
    pub fn output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }
    /// Required: maps `Tool::invoke` input → seed slot writes.
    pub fn seed_adapter(mut self, adapter: SeedAdapter) -> Self {
        self.seed_adapter = Some(adapter);
        self
    }
    /// Required: maps terminal `SlotMap` → `Tool::invoke` return.
    pub fn output_adapter(mut self, adapter: OutputAdapter) -> Self {
        self.output_adapter = Some(adapter);
        self
    }

    /// Finalise the builder.
    pub fn build(self) -> Result<FlowAsTool, FlowAsToolBuildError> {
        let flow_id = self
            .flow_id
            .ok_or(FlowAsToolBuildError::MissingField("flow_id"))?;
        let revision = self
            .revision
            .ok_or(FlowAsToolBuildError::MissingField("revision"))?;
        let topology = self
            .topology
            .ok_or(FlowAsToolBuildError::MissingField("topology"))?;
        if self.terminal_slots.is_empty() {
            return Err(FlowAsToolBuildError::MissingField("terminal_slots"));
        }
        let engine = self
            .engine
            .ok_or(FlowAsToolBuildError::MissingField("engine"))?;
        let tool_id = self
            .tool_id
            .ok_or(FlowAsToolBuildError::MissingField("tool_id"))?;
        let name = self
            .name
            .ok_or(FlowAsToolBuildError::MissingField("name"))?;
        let description = self
            .description
            .ok_or(FlowAsToolBuildError::MissingField("description"))?;
        let input_schema = self
            .input_schema
            .ok_or(FlowAsToolBuildError::MissingField("input_schema"))?;
        let output_schema = self
            .output_schema
            .ok_or(FlowAsToolBuildError::MissingField("output_schema"))?;
        let seed_adapter = self
            .seed_adapter
            .ok_or(FlowAsToolBuildError::MissingField("seed_adapter"))?;
        let output_adapter = self
            .output_adapter
            .ok_or(FlowAsToolBuildError::MissingField("output_adapter"))?;
        Ok(FlowAsTool {
            flow_id,
            revision,
            topology,
            terminal_slots: self.terminal_slots,
            engine,
            tool_id,
            name,
            description,
            input_schema,
            output_schema,
            seed_adapter,
            output_adapter,
        })
    }
}

// ---------------------------------------------------------------------------
// FlowAsService — body lands in stage 8.
// ---------------------------------------------------------------------------

/// Wraps a flow as a `starter_spi::service::Service` (R9).
///
/// Stage 7 ships [`FlowAsTool`]; the [`FlowAsService`] body lands
/// in stage 8 per the job WORKFLOW per-stage table. The struct
/// stays empty until then so consumers can name the type in
/// `where` bounds without inheriting a half-built impl.
pub struct FlowAsService {
    // Phase 3 stage 8 — fields per R9 + D-F3.5
    // (subscribe-on-start lifecycle, dedup-key resolution, etc.).
    // Left absent rather than `todo!()` so a half-finished impl
    // cannot leak into consumers.
}
