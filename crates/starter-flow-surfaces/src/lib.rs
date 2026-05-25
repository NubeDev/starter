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

pub mod clock;
pub mod flow_registry;
pub mod service;

pub use flow_registry::{
    register::FlowRegistration, resolve::FromRegistryError, FlowRegistry, FlowRegistryError,
    RegisteredFlow, ResolvedFlow,
};

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::timeout;

use starter_flow::engine::Engine;
use starter_flow::propagator::FlowTopology;
use starter_flow::run::{
    FlowRunner, FlowRunnerConfig, InMemoryRunStore, RunCancel, RunSpec, RunStore as FlowRunStore,
};
use starter_flow::state::RunStatus;
use starter_flow_spi::flow::{DedupKey, FlowEvent, FlowId, FlowRevisionId};
use starter_flow_spi::node::{KindId, SlotMap, SlotRef, SlotValue};
use starter_flow_spi::{Cancel, Principal};
use starter_spi::error::{Error as SpiError, Result as SpiResult};
use starter_spi::service::{Event, EventSink, Service, ServiceContext, ServiceHandle};
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
        if let Some(node_state) = self.engine.node_state_store() {
            runner = runner.with_node_state_store(node_state.clone());
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
        // The same loop also forwards every event into the
        // engine's optional `FlowEventSink` so SSE subscribers
        // (e.g. rubix-agent's `FlowSubscriptionRegistry`) see
        // live `NodeEmitted` / `RunCompleted` frames from runs
        // started through this surface.
        //
        // We consume `handle.initial_rx` — the pre-subscribed
        // receiver the runner set up *before* spawning the
        // coordinator — rather than calling
        // `handle.events_tx.subscribe()` here. A fresh
        // `subscribe()` after `runner.start(..)` returns races the
        // coordinator: short flows (the `tick-counter` reference
        // demo finishes in microseconds) emit every
        // `NodeEmitted` / `RunCompleted` before this task is
        // polled, and `broadcast` only delivers events sent
        // *after* a receiver was created. Using `initial_rx`
        // closes that race and guarantees the optional
        // `FlowEventSink` (the rubix SSE fan-out) sees every
        // frame.
        let node_failure: Arc<Mutex<Option<(String, String)>>> = Arc::new(Mutex::new(None));
        let mut events_rx = handle.initial_rx;
        let node_failure_for_task = node_failure.clone();
        let sink_for_task = self.engine.event_sink().cloned();
        let flow_for_task = self.flow_id.clone();
        let watcher = tokio::spawn(async move {
            while let Ok(ev) = events_rx.recv().await {
                if let FlowEvent::NodeFailed { node, error, .. } = &ev {
                    let mut slot = node_failure_for_task
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    if slot.is_none() {
                        *slot = Some((node.to_string(), error.clone()));
                    }
                }
                if let Some(sink) = sink_for_task.as_ref() {
                    sink.publish(&flow_for_task, ev);
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
// FlowAsService — SCOPE R9 + D-F3.5 + D-F3.12.
// ---------------------------------------------------------------------------

/// Adapter mapping one incoming [`Event`] to the per-event seed
/// slot writes the wrapped flow's propagator consumes.
///
/// The output mirrors the per-tool [`SeedAdapter`] but is keyed on
/// the [`Event`] envelope (`kind` + `payload`) rather than a raw
/// JSON input. Returning an empty vec is a valid "no-op event"
/// signal — the service logs and continues without starting a
/// new run.
pub type ServiceSeedAdapter =
    Arc<dyn Fn(&Event) -> Vec<(SlotRef, SlotValue)> + Send + Sync + 'static>;

/// Factory yielding a fresh subscriber to the upstream event
/// stream every time [`Service::start`] runs.
///
/// `tokio::sync::broadcast::Receiver` is `!Clone`, so the
/// subscription is materialised on each `start` by calling this
/// closure. The closure typically captures an `Arc<broadcast::Sender<Event>>`
/// and returns `sender.subscribe()`. Tests construct one inline;
/// production wiring lives in whichever crate owns the upstream
/// `EventSink` (typically a service that publishes via the
/// blanket `broadcast::Sender<E>` impl per
/// [`starter_spi::service::broadcast`]).
pub type EventSubscriber = Arc<dyn Fn() -> broadcast::Receiver<Event> + Send + Sync + 'static>;

/// Wraps a flow as a `starter_spi::service::Service` (SCOPE R9 +
/// D-F3.5).
///
/// Per R9: a flow with a `trigger.webhook` entry node and an
/// `ai-agent` body and a `tool-call` output is, simultaneously,
/// a webhook endpoint, an MCP tool (via [`FlowAsTool`]), and an
/// event-driven service (via [`FlowAsService`]). The author
/// wrote one flow.
///
/// Per D-F3.5: subscription happens inside
/// [`Service::start`] (not at construction time) so the
/// `ServiceContext::shutdown` watch is wired through to the
/// worker task; the worker exits cleanly when the watch flips
/// to `true` and the spawned `JoinHandle` resolves with `Ok(())`.
///
/// Per D-F3.12: the dedup key for each incoming event is
/// resolved as `EventSink::dedup_key(kind, payload)` first, with
/// a blake3-over-canonical-bytes fallback when the sink declines
/// to provide one. The key is threaded through
/// [`RunSpec::with_dedup_key`] so the SPI `RunStore::start` call
/// persists it under the `UNIQUE (service_name, dedup_key)`
/// partial index; re-deliveries hit
/// `RunStore::find_by_dedup_key` and short-circuit with a
/// [`FlowEvent::DedupShortCircuit`] emission instead of starting
/// a duplicate run.
///
/// Degraded-engine policy (stage 8 decision): the service stays
/// alive when [`Engine`] is degraded; per-event
/// `FlowRunner::start` rejections surface as a `tracing::warn!`
/// event with the dedup key for downstream investigation. The
/// service does **not** queue events (the engine's own degraded
/// queue is per-run; per-service queuing would invite unbounded
/// memory growth on a long-degraded backend). Re-delivery from
/// the upstream transport is the recovery path; D-F3.12 dedup
/// makes that safe.
pub struct FlowAsService {
    flow_id: FlowId,
    revision: FlowRevisionId,
    topology: Arc<FlowTopology>,
    terminal_slots: Vec<SlotRef>,
    engine: Arc<Engine>,
    service_id: KindId,
    name: String,
    description: String,
    event_sink: Arc<dyn EventSink>,
    event_subscriber: EventSubscriber,
    seed_adapter: ServiceSeedAdapter,
    principal: Principal,
}

impl FlowAsService {
    /// Start a fresh [`FlowAsServiceBuilder`].
    pub fn builder() -> FlowAsServiceBuilder {
        FlowAsServiceBuilder::default()
    }

    /// Borrow the [`FlowId`] this wrapper invokes.
    pub fn flow_id(&self) -> &FlowId {
        &self.flow_id
    }

    /// Borrow the reverse-DNS [`KindId`] this service is
    /// registered under (R10).
    pub fn service_id(&self) -> &KindId {
        &self.service_id
    }

    /// Borrow the human-facing description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Borrow the human-facing name (tracing/metrics label).
    pub fn display_name(&self) -> &str {
        &self.name
    }
}

#[async_trait]
impl Service for FlowAsService {
    fn name(&self) -> &'static str {
        // The trait wants `&'static str` for tracing/metrics
        // labels; the runtime `String` `name` lives on the
        // builder for human display. The `service_id.as_str()`
        // is the reverse-DNS stable id, but it's not `'static`.
        // Fall back to the constant the service crate name
        // makes available — operators correlate runs via the
        // tracing field `service = %self.service_id.as_str()`
        // emitted on every per-event log line.
        "starter.flow-as-service"
    }

    async fn start(&self, ctx: ServiceContext) -> SpiResult<ServiceHandle> {
        let mut rx = (self.event_subscriber)();
        let mut shutdown = ctx.shutdown.clone();
        let me = FlowAsServiceWorkerHandle::new(self);

        let join: JoinHandle<SpiResult<()>> = tokio::spawn(async move {
            tracing::info!(
                service = %me.service_id.as_str(),
                "flow_as_service.worker.started",
            );
            loop {
                tokio::select! {
                    biased;
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                    recv = rx.recv() => {
                        match recv {
                            Ok(event) => {
                                if let Err(e) = me.handle_event(event).await {
                                    tracing::warn!(
                                        service = %me.service_id.as_str(),
                                        error = ?e,
                                        "flow_as_service.handle_event_failed",
                                    );
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!(
                                    service = %me.service_id.as_str(),
                                    dropped = n,
                                    "flow_as_service.subscriber_lagged",
                                );
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                tracing::info!(
                                    service = %me.service_id.as_str(),
                                    "flow_as_service.subscription_closed",
                                );
                                break;
                            }
                        }
                    }
                }
            }
            tracing::info!(
                service = %me.service_id.as_str(),
                "flow_as_service.worker.exited",
            );
            Ok(())
        });

        Ok(ServiceHandle::new(join))
    }
}

/// Per-worker borrow of [`FlowAsService`] inputs. The
/// [`Service::start`] task takes `&self`, but the spawned worker
/// needs `'static` access; this handle clones the small `Arc`-
/// shaped fields once at start time so the worker can `await`
/// without retaining a reference into `self`.
struct FlowAsServiceWorkerHandle {
    flow_id: FlowId,
    revision: FlowRevisionId,
    topology: Arc<FlowTopology>,
    terminal_slots: Vec<SlotRef>,
    engine: Arc<Engine>,
    service_id: KindId,
    event_sink: Arc<dyn EventSink>,
    seed_adapter: ServiceSeedAdapter,
    principal: Principal,
}

impl FlowAsServiceWorkerHandle {
    fn new(svc: &FlowAsService) -> Self {
        Self {
            flow_id: svc.flow_id.clone(),
            revision: svc.revision,
            topology: svc.topology.clone(),
            terminal_slots: svc.terminal_slots.clone(),
            engine: svc.engine.clone(),
            service_id: svc.service_id.clone(),
            event_sink: svc.event_sink.clone(),
            seed_adapter: svc.seed_adapter.clone(),
            principal: svc.principal.clone(),
        }
    }

    async fn handle_event(&self, event: Event) -> SpiResult<Option<()>> {
        // Reconstruct a minimal `FlowAsService`-shaped view by
        // delegating: rather than duplicate the body, build a
        // throwaway `FlowAsService` view? Simpler: inline the
        // body here, since the worker has all the same fields.
        let dedup = {
            let key = match self.event_sink.dedup_key(&event.kind, &event.payload) {
                Some(k) => k,
                None => {
                    let mut hasher = blake3::Hasher::new();
                    hasher.update(self.service_id.as_str().as_bytes());
                    hasher.update(b"\0");
                    hasher.update(event.kind.as_bytes());
                    hasher.update(b"\0");
                    if let Ok(bytes) = serde_json::to_vec(&event.payload) {
                        hasher.update(&bytes);
                    }
                    hasher.finalize().to_hex().to_string()
                }
            };
            DedupKey::new(self.service_id.as_str(), key)
        };

        if let Some(spi) = self.engine.run_store() {
            match spi.find_by_dedup_key(&dedup.service_name, &dedup.key).await {
                Ok(Some(prior_run_id)) => {
                    let ev = FlowEvent::DedupShortCircuit { prior_run_id };
                    tracing::info!(
                        service = %self.service_id.as_str(),
                        dedup_key = %dedup.key,
                        ?ev,
                        "flow_as_service.dedup_short_circuit",
                    );
                    return Ok(None);
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        service = %self.service_id.as_str(),
                        error = %e,
                        "flow_as_service.find_by_dedup_key failed; falling through to start",
                    );
                }
            }
        }

        let seeds = (self.seed_adapter)(&event);
        if seeds.is_empty() {
            tracing::debug!(
                service = %self.service_id.as_str(),
                kind = %event.kind,
                "flow_as_service.seed_adapter returned no seeds; skipping",
            );
            return Ok(None);
        }
        let spec = RunSpec::new(
            self.flow_id.clone(),
            self.revision,
            self.topology.clone(),
            seeds,
            self.terminal_slots.clone(),
        )
        .with_principal(self.principal.clone())
        .with_dedup_key(dedup.clone());

        let mut runner = FlowRunner::new(
            self.engine.store.clone(),
            Arc::new(InMemoryRunStore::new()) as Arc<dyn FlowRunStore>,
        )
        .with_health_handle(self.engine.health_handle())
        .with_config(FlowRunnerConfig::default());
        if let Some(spi_store) = self.engine.run_store() {
            runner = runner.with_spi_run_store(spi_store.clone());
        }
        if let Some(node_state) = self.engine.node_state_store() {
            runner = runner.with_node_state_store(node_state.clone());
        }

        let handle = match runner.start(spec, SlotMap::new()).await {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!(
                    service = %self.service_id.as_str(),
                    dedup_key = %dedup.key,
                    error = %e,
                    "flow_as_service.start_refused",
                );
                return Ok(None);
            }
        };

        match handle.join.await {
            Ok(status) => {
                tracing::debug!(
                    service = %self.service_id.as_str(),
                    run = %handle.run,
                    ?status,
                    "flow_as_service.run_terminated",
                );
            }
            Err(join_err) => {
                tracing::warn!(
                    service = %self.service_id.as_str(),
                    error = %join_err,
                    "flow_as_service.run_join_failed",
                );
            }
        }
        Ok(Some(()))
    }
}

// ---------------------------------------------------------------------------
// FlowAsServiceBuilder
// ---------------------------------------------------------------------------

/// Builder for [`FlowAsService`]. Every field is required;
/// missing fields surface as
/// [`FlowAsServiceBuildError::MissingField`] from
/// [`FlowAsServiceBuilder::build`].
#[must_use = "FlowAsServiceBuilder does nothing until `.build()` is called"]
#[derive(Default)]
pub struct FlowAsServiceBuilder {
    flow_id: Option<FlowId>,
    revision: Option<FlowRevisionId>,
    topology: Option<Arc<FlowTopology>>,
    terminal_slots: Vec<SlotRef>,
    engine: Option<Arc<Engine>>,
    service_id: Option<KindId>,
    name: Option<String>,
    description: Option<String>,
    event_sink: Option<Arc<dyn EventSink>>,
    event_subscriber: Option<EventSubscriber>,
    seed_adapter: Option<ServiceSeedAdapter>,
    principal: Option<Principal>,
}

/// Error from [`FlowAsServiceBuilder::build`].
#[derive(Debug)]
#[non_exhaustive]
pub enum FlowAsServiceBuildError {
    /// One or more required fields were not set; the value
    /// names the first missing field encountered.
    MissingField(&'static str),
}

impl std::fmt::Display for FlowAsServiceBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(name) => write!(f, "FlowAsService builder missing field: {name}"),
        }
    }
}

impl std::error::Error for FlowAsServiceBuildError {}

impl FlowAsServiceBuilder {
    /// Required: the flow this service wraps.
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
    /// Required (non-empty): the terminal slots gathered into
    /// the per-run output map.
    pub fn terminal_slots(mut self, slots: Vec<SlotRef>) -> Self {
        self.terminal_slots = slots;
        self
    }
    /// Required: the engine handle.
    pub fn engine(mut self, engine: Arc<Engine>) -> Self {
        self.engine = Some(engine);
        self
    }
    /// Required: the reverse-DNS service id (R10). Used as the
    /// `DedupKey.service_name` for D-F3.12.
    pub fn service_id(mut self, id: KindId) -> Self {
        self.service_id = Some(id);
        self
    }
    /// Required: human-readable service name (tracing/metrics
    /// labels, registry display).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    /// Required: human description of what the service does.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    /// Required: the upstream [`EventSink`]. Consulted for its
    /// optional [`EventSink::dedup_key`] override per D-F3.12;
    /// otherwise an inert reference held only so this wrapper
    /// can ask the same sink about future events.
    pub fn event_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }
    /// Required: factory for fresh subscribers to the upstream
    /// event stream. Called once per [`Service::start`]
    /// invocation (D-F3.5 subscribe-on-start lifecycle).
    pub fn event_subscriber(mut self, subscriber: EventSubscriber) -> Self {
        self.event_subscriber = Some(subscriber);
        self
    }
    /// Required: maps an incoming [`Event`] into seed slot writes.
    /// Returning an empty `Vec` means "no-op event; skip".
    pub fn seed_adapter(mut self, adapter: ServiceSeedAdapter) -> Self {
        self.seed_adapter = Some(adapter);
        self
    }
    /// Required: the [`Principal`] every per-event run is
    /// recorded under (retires the stage-5 `system/Admin`
    /// default for service-driven runs).
    pub fn principal(mut self, principal: Principal) -> Self {
        self.principal = Some(principal);
        self
    }

    /// Finalise the builder.
    pub fn build(self) -> Result<FlowAsService, FlowAsServiceBuildError> {
        let flow_id = self
            .flow_id
            .ok_or(FlowAsServiceBuildError::MissingField("flow_id"))?;
        let revision = self
            .revision
            .ok_or(FlowAsServiceBuildError::MissingField("revision"))?;
        let topology = self
            .topology
            .ok_or(FlowAsServiceBuildError::MissingField("topology"))?;
        if self.terminal_slots.is_empty() {
            return Err(FlowAsServiceBuildError::MissingField("terminal_slots"));
        }
        let engine = self
            .engine
            .ok_or(FlowAsServiceBuildError::MissingField("engine"))?;
        let service_id = self
            .service_id
            .ok_or(FlowAsServiceBuildError::MissingField("service_id"))?;
        let name = self
            .name
            .ok_or(FlowAsServiceBuildError::MissingField("name"))?;
        let description = self
            .description
            .ok_or(FlowAsServiceBuildError::MissingField("description"))?;
        let event_sink = self
            .event_sink
            .ok_or(FlowAsServiceBuildError::MissingField("event_sink"))?;
        let event_subscriber = self
            .event_subscriber
            .ok_or(FlowAsServiceBuildError::MissingField("event_subscriber"))?;
        let seed_adapter = self
            .seed_adapter
            .ok_or(FlowAsServiceBuildError::MissingField("seed_adapter"))?;
        let principal = self
            .principal
            .ok_or(FlowAsServiceBuildError::MissingField("principal"))?;
        Ok(FlowAsService {
            flow_id,
            revision,
            topology,
            terminal_slots: self.terminal_slots,
            engine,
            service_id,
            name,
            description,
            event_sink,
            event_subscriber,
            seed_adapter,
            principal,
        })
    }
}
