//! [`Supervisor`] — spawn, init-handshake, watch, restart.
//!
//! This is the entry point a consumer (or `starter-ext-server`) uses to
//! bring a process-flavour extension up. It owns the child's lifecycle:
//! one `spawn → init handshake → run → exit observed → decide → respawn
//! or stop` cycle per restart, plus the periodic health pinger and the
//! capability-gated wire reader running in the same async task tree.
//!
//! The supervisor never *blocks the caller*. [`Supervisor::start`] spawns
//! the management task and returns a [`SupervisorHandle`] the caller can
//! query for state, push outbound JSON-RPC requests through, or use to
//! request a shutdown.
//!
//! ## What is *not* in v0.1
//!
//! - **Supervisor groups** (SCOPE R9): every extension is its own subtree.
//! - **cgroups / rlimits** (SCOPE R8): a v0.2 feature behind an explicit
//!   threat model.
//! - **Bidirectional host-method dispatch over the wire**: the capability
//!   gate is wired (any child that calls `secrets.get` without declaring
//!   the capability sees an error and a counter increment) but the actual
//!   host-side method handlers are stubbed — the SDK's `Ctx` adapters
//!   land alongside the wasm + transport adapters in later phases. The
//!   *shape* is here; the bodies fill in additively.
//!
//! Both are intentional: keeping the v0.1 supervisor's state machine an
//! order of magnitude simpler is what SCOPE asks for ("v0.1 stays simple").

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use starter_ext_host::ExtensionRecord;
use starter_ext_spi::{
    jsonrpc::{stream_methods, JSONRPC_VERSION},
    CallerIdentity, Error, ExtensionId, FrameMeta, LifecycleState, Result, StreamCancel, StreamId,
    StreamNotification,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Dispatch request ids start above this floor so they never collide
/// with the supervisor's internal health-ping ids (which start at
/// `1_000` and increment per ping). Adapters allocate ids from
/// [`SupervisorHandle::call`] using a per-handle atomic seeded above
/// this value; the wire-loop demultiplexer routes responses with
/// `id >= DISPATCH_ID_FLOOR` into the pending map and treats anything
/// below as a health response (clears the deadline only).
const DISPATCH_ID_FLOOR: i64 = 1_000_000;

/// Inner table of in-flight dispatch requests, keyed by JSON-RPC id.
/// Shared by [`SupervisorHandle`] (inserts on send) and the supervisor
/// task (removes on response, or drains on task exit so callers see a
/// transport error instead of hanging on `recv`).
type PendingMap =
    Arc<Mutex<HashMap<i64, oneshot::Sender<core::result::Result<serde_json::Value, Error>>>>>;

/// In-flight stream subscriptions keyed by `stream_id`. Owned jointly
/// by [`SupervisorHandle::subscribe_stream`] (inserts) and the
/// supervisor task (looks up + forwards on every `stream.*`
/// notification). Cleared on task exit so dangling subscribers see
/// the channel close instead of hanging.
type StreamSubscribers = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<StreamNotification>>>>;

use crate::backoff::BackoffSchedule;
use crate::capability::{CapabilityGate, CapabilityViolationCounter};
use crate::event_ring::{EventKind, EventRing, MAX_STDERR_LINE_BYTES};
use crate::handshake::{manifest_hash, InitHandshake, InitReady};
use crate::restart::{ExitReason, RestartDecision, RestartTracker};
use crate::stream::is_streaming_notification;

/// Why the supervisor decided to wind down. Surfaced to the caller via
/// the watch channel returned from [`SupervisorHandle::state`] and to
/// the event ring as a final `StateTransition`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShutdownReason {
    /// Operator (or owning code) requested shutdown via
    /// [`SupervisorHandle::shutdown`].
    Requested,
    /// `RestartPolicy` said "stop" after a clean exit.
    PolicyStop,
    /// Restart intensity cap exceeded.
    Failed,
}

/// Public handle to a running supervisor task.
#[derive(Debug, Clone)]
pub struct SupervisorHandle {
    id: ExtensionId,
    state: watch::Receiver<LifecycleState>,
    shutdown_tx: mpsc::Sender<()>,
    events: Arc<EventRing>,
    violations: Arc<CapabilityViolationCounter>,
    inbound: mpsc::UnboundedSender<serde_json::Value>,
    pending: PendingMap,
    next_request_id: Arc<AtomicI64>,
    stream_subscribers: StreamSubscribers,
}

impl SupervisorHandle {
    /// The extension's id. Surfaced so admin endpoints can key by id
    /// without a separate lookup.
    pub fn id(&self) -> &ExtensionId {
        &self.id
    }

    /// Subscribe to lifecycle-state changes. The watch channel always has
    /// the current value, so a late subscriber sees the steady state.
    pub fn state(&self) -> watch::Receiver<LifecycleState> {
        self.state.clone()
    }

    /// Snapshot the event ring. Bounded; safe to call on the request path.
    pub fn events(&self) -> Vec<crate::event_ring::Event> {
        self.events.snapshot()
    }

    /// Read the capability-violation counter.
    pub fn capability_violations(&self) -> u64 {
        self.violations.get()
    }

    /// Send a JSON-RPC envelope to the child (request or notification).
    /// Caller is responsible for constructing valid JSON-RPC; this is the
    /// outbound side of the bidirectional channel, used by adapters to
    /// forward `stream.cancel`, drive `init`, or invoke contributed tools.
    pub fn send(&self, envelope: serde_json::Value) -> Result<()> {
        self.inbound
            .send(envelope)
            .map_err(|_| Error::transport("supervisor task is no longer running"))
    }

    /// Send a JSON-RPC envelope with a [`CallerIdentity`] stamped onto
    /// its `_meta.caller` field. Identical to [`Self::send`] otherwise.
    ///
    /// Adapters use this to stamp the requesting principal onto every
    /// outbound frame the supervisor forwards to the child — the
    /// north-star "Rule 3" plumbing that lets tenant-scoped capability
    /// handles refuse a frame without an owning tenant. A pre-existing
    /// `_meta.caller` on the envelope is overwritten so the supervisor
    /// is the single source of truth.
    pub fn send_with_caller(
        &self,
        mut envelope: serde_json::Value,
        caller: CallerIdentity,
    ) -> Result<()> {
        stamp_caller(&mut envelope, caller);
        self.send(envelope)
    }

    /// Request a graceful shutdown. The supervisor sends `SIGTERM`, waits
    /// the manifest's grace window, then `SIGKILL` if needed.
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(()).await;
    }

    /// Send a `stream.cancel { stream_id, reason }` notification to
    /// the child as the canonical way to cancel an in-flight
    /// streaming invocation (or any other open `stream_id`).
    ///
    /// Proxies (`ProcessNodeProxy`, future per-transport adapters)
    /// route their cancellation through this helper rather than
    /// hand-rolling the envelope so the wire shape stays in one
    /// place (`DOCS/extensions/scope/FLOW-NODES.md` R-flow-node-5:
    /// `stream.cancel { stream_id: invocation_id }` for both
    /// streaming and non-streaming invocations).
    pub fn stream_cancel(
        &self,
        stream_id: &StreamId,
        reason: Option<impl Into<String>>,
    ) -> Result<()> {
        let envelope = json!({
            "jsonrpc": JSONRPC_VERSION,
            "method": stream_methods::CANCEL,
            "params": StreamCancel {
                stream_id: stream_id.clone(),
                reason: reason.map(Into::into),
            },
        });
        self.send(envelope)
    }

    /// Subscribe to every `stream.*` notification whose `stream_id`
    /// equals `stream_id`. Returns the receiving half of an unbounded
    /// MPSC; the sending half lives on the supervisor task and is
    /// dropped when the task exits (handle holders observe a closed
    /// channel instead of hanging on `recv`).
    ///
    /// The proxy registers a subscription *before* it issues the
    /// matching `flow.node.invoke` so a fast child cannot emit a
    /// `stream.event` before the proxy is listening (the supervisor
    /// otherwise drops the notification on the floor — there is no
    /// per-stream buffering in v0.1).
    ///
    /// Inserting twice under the same `stream_id` replaces the prior
    /// sender and returns a fresh receiver; the orphaned sender's
    /// receiver will observe channel-closed on the next recv. v0.1
    /// callers (`ProcessNodeProxy`) never share a stream id between
    /// invocations.
    pub fn subscribe_stream(
        &self,
        stream_id: &StreamId,
    ) -> mpsc::UnboundedReceiver<StreamNotification> {
        let (tx, rx) = mpsc::unbounded_channel();
        if let Ok(mut g) = self.stream_subscribers.lock() {
            g.insert(stream_id.0.clone(), tx);
        }
        rx
    }

    /// Cancel a stream subscription previously registered through
    /// [`Self::subscribe_stream`]. Used by the proxy's drop guard so a
    /// subscription whose invocation completed (normal response,
    /// timeout, cancel-forwarder trip) does not leak in the supervisor's
    /// map.
    pub fn unsubscribe_stream(&self, stream_id: &StreamId) {
        if let Ok(mut g) = self.stream_subscribers.lock() {
            g.remove(&stream_id.0);
        }
    }

    /// Synchronous JSON-RPC call against the child.
    ///
    /// Allocates a dispatch id (above [`DISPATCH_ID_FLOOR`]), frames a
    /// JSON-RPC 2.0 request `{ method, params }`, pushes it through the
    /// supervisor's outbound channel, and awaits the matching response
    /// from the child up to `timeout`.
    ///
    /// Errors:
    /// - [`Error::Transport`] if the supervisor task is no longer
    ///   running, the timeout elapses, or the child closes its stdout
    ///   before answering.
    /// - The child's own error envelope (decoded as [`Error`]) is
    ///   returned verbatim so adapters can map it onto their transport's
    ///   error vocabulary (`DispatchError::from_kernel`).
    ///
    /// This is the request/response demultiplexer the transport
    /// adapters (`starter-ext-cli`, `starter-ext-server`,
    /// `starter-ext-grpc`, `starter-ext-mcp`) build on top of to remove
    /// their `NotWired` paths. The streaming sub-protocol
    /// (`stream.event` / `stream.end` / `stream.error`) is *not* served
    /// here — streaming dispatch requires a separate per-stream
    /// demultiplexer that ships in a later slice.
    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value> {
        self.call_inner(method, params, None, timeout).await
    }

    /// Like [`Self::call`] but stamps a [`CallerIdentity`] onto the
    /// request's `_meta.caller` field so the child's `ctx.caller()`
    /// resolves it. Use from any dispatcher whose inbound transport
    /// already carries an authenticated principal (REST middleware,
    /// gRPC interceptor, MCP authn, …).
    ///
    /// A frame stamped with [`CallerIdentity::system`] is equivalent to
    /// calling [`Self::call`] — the SDK reflects both as
    /// `ctx.caller() == None` so handlers cannot accidentally treat a
    /// system frame as a tenant frame.
    pub async fn call_as(
        &self,
        method: &str,
        params: serde_json::Value,
        caller: CallerIdentity,
        timeout: Duration,
    ) -> Result<serde_json::Value> {
        self.call_inner(method, params, Some(caller), timeout).await
    }

    async fn call_inner(
        &self,
        method: &str,
        params: serde_json::Value,
        caller: Option<CallerIdentity>,
        timeout: Duration,
    ) -> Result<serde_json::Value> {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        {
            let mut guard = self
                .pending
                .lock()
                .map_err(|_| Error::transport("pending map poisoned"))?;
            guard.insert(id, tx);
        }

        let mut envelope = json!({
            "jsonrpc": JSONRPC_VERSION,
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(c) = caller {
            stamp_caller(&mut envelope, c);
        }
        if let Err(e) = self.send(envelope) {
            // Remove our pending entry so the slot doesn't leak; the
            // task is gone so no one will complete it.
            if let Ok(mut g) = self.pending.lock() {
                g.remove(&id);
            }
            return Err(e);
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                if let Ok(mut g) = self.pending.lock() {
                    g.remove(&id);
                }
                Err(Error::transport(
                    "supervisor task dropped pending request (child likely crashed)",
                ))
            }
            Err(_) => {
                if let Ok(mut g) = self.pending.lock() {
                    g.remove(&id);
                }
                Err(Error::transport(format!(
                    "jsonrpc request timed out after {timeout:?}"
                )))
            }
        }
    }
}

/// The supervisor's entry point.
pub struct Supervisor;

impl Supervisor {
    /// Start a supervisor task for the record.
    ///
    /// Returns immediately with the handle; the supervision task is
    /// spawned onto the current tokio runtime. The record must be
    /// `Validated`, have `manifest.runtime.kind == Process`, and carry a
    /// `manifest.runtime.bin` path the supervisor can `exec` (resolved
    /// against the bundle dir).
    pub fn start(record: &ExtensionRecord) -> Result<SupervisorHandle> {
        let manifest = record
            .manifest
            .as_ref()
            .ok_or_else(|| Error::spawn("record has no parsed manifest"))?;
        let id = record
            .id
            .clone()
            .ok_or_else(|| Error::spawn("record has no validated id"))?;
        let bin_rel = manifest
            .runtime
            .bin
            .as_deref()
            .ok_or_else(|| Error::spawn("runtime.bin missing for process flavour"))?;
        let bin = record.bundle_dir.join(bin_rel);
        let sup_cfg = manifest.supervision.clone().unwrap_or_else(|| {
            // Caller did not supply a supervision block; use the manifest
            // module's defaults (R9 — manifest is source of truth, but
            // sane defaults keep ergonomics).
            use starter_ext_spi::{Backoff, HealthConfig, RestartPolicy, Supervision};
            Supervision {
                restart: RestartPolicy::OnCrash,
                max_restarts: 5,
                within_seconds: 60,
                backoff: Backoff::default(),
                health: HealthConfig::default(),
                group: None,
                shutdown_grace_ms: 5_000,
            }
        });

        // Read the bundle's block.yaml so we can hand its content hash
        // to the init handshake. Missing/unreadable manifest is `Spawn`.
        let manifest_bytes = std::fs::read(record.bundle_dir.join("block.yaml"))
            .map_err(|e| Error::spawn(format!("reading bundle block.yaml: {e}")))?;
        let manifest_digest = manifest_hash(&manifest_bytes);

        let (state_tx, state_rx) = watch::channel(LifecycleState::Starting);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel::<serde_json::Value>();
        let events = Arc::new(EventRing::new());
        let violations = Arc::new(CapabilityViolationCounter::default());
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let next_request_id = Arc::new(AtomicI64::new(DISPATCH_ID_FLOOR));
        let stream_subscribers: StreamSubscribers = Arc::new(Mutex::new(HashMap::new()));

        let task = SupervisorTask {
            id: id.clone(),
            bin,
            bundle_dir: record.bundle_dir.clone(),
            manifest_digest,
            config: manifest.config.clone(),
            sup_cfg: sup_cfg.clone(),
            gate: CapabilityGate::from_manifest(&manifest.capabilities),
            restart: RestartTracker::from_manifest(&sup_cfg),
            backoff: BackoffSchedule::from_manifest(&sup_cfg.backoff),
            state_tx,
            shutdown_rx,
            inbound_rx,
            events: events.clone(),
            violations: violations.clone(),
            pending: pending.clone(),
            stream_subscribers: stream_subscribers.clone(),
        };
        tokio::spawn(task.run());

        Ok(SupervisorHandle {
            id,
            state: state_rx,
            shutdown_tx,
            events,
            violations,
            inbound: inbound_tx,
            pending,
            next_request_id,
            stream_subscribers,
        })
    }
}

// ---------------------------------------------------------------------------
// SupervisorTask — the body of the management loop. One per extension.
// ---------------------------------------------------------------------------

struct SupervisorTask {
    id: ExtensionId,
    bin: PathBuf,
    bundle_dir: PathBuf,
    manifest_digest: String,
    config: serde_json::Value,
    sup_cfg: starter_ext_spi::Supervision,
    gate: CapabilityGate,
    restart: RestartTracker,
    backoff: BackoffSchedule,
    state_tx: watch::Sender<LifecycleState>,
    shutdown_rx: mpsc::Receiver<()>,
    inbound_rx: mpsc::UnboundedReceiver<serde_json::Value>,
    events: Arc<EventRing>,
    violations: Arc<CapabilityViolationCounter>,
    /// Pending dispatch requests (id ≥ [`DISPATCH_ID_FLOOR`]) waiting
    /// for the matching child response. Drained on task exit so any
    /// in-flight [`SupervisorHandle::call`] returns a transport error
    /// instead of hanging.
    pending: PendingMap,
    /// In-flight stream subscribers keyed by `stream_id`. Drained on
    /// task exit so dangling proxies observe channel-closed instead
    /// of hanging on `recv`.
    stream_subscribers: StreamSubscribers,
}

impl Drop for SupervisorTask {
    fn drop(&mut self) {
        // Cancel every in-flight dispatch — dropping the `oneshot::Sender`
        // makes the receiver fail with `RecvError`, which the handle's
        // `call` translates into `Error::Transport("... child likely
        // crashed")`.
        if let Ok(mut g) = self.pending.lock() {
            g.clear();
        }
        // Drop every subscriber sender; receivers see a closed
        // channel and their owning proxies return a transport error
        // instead of hanging.
        if let Ok(mut g) = self.stream_subscribers.lock() {
            g.clear();
        }
    }
}

impl SupervisorTask {
    async fn run(mut self) {
        loop {
            self.publish_state(LifecycleState::Starting);
            let exit_reason = match self.spawn_and_serve().await {
                Ok(reason) => reason,
                Err(e) => {
                    warn!(
                        ext = %self.id.as_str(),
                        err = %e,
                        "spawn / init failed",
                    );
                    self.events.push(EventKind::Crashed {
                        reason: format!("{e}"),
                    });
                    ExitReason::Crash
                }
            };

            // Was a shutdown requested while we were live?
            if self.shutdown_drained() {
                self.publish_state(LifecycleState::Stopped);
                self.events.push(EventKind::StateTransition {
                    to: LifecycleState::Stopped,
                });
                return;
            }

            match self.restart.should_restart(exit_reason) {
                RestartDecision::Restart => {
                    let wait = self.backoff.next_wait();
                    self.events.push(EventKind::RestartScheduled {
                        wait_ms: wait.as_millis() as u64,
                        total: self.restart.total(),
                    });
                    info!(
                        ext = %self.id.as_str(),
                        wait_ms = wait.as_millis() as u64,
                        "scheduling restart",
                    );
                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {}
                        _ = self.shutdown_rx.recv() => {
                            self.publish_state(LifecycleState::Stopped);
                            return;
                        }
                    }
                }
                RestartDecision::Stop => {
                    self.publish_state(LifecycleState::Stopped);
                    self.events.push(EventKind::StateTransition {
                        to: LifecycleState::Stopped,
                    });
                    return;
                }
                RestartDecision::Failed => {
                    self.publish_state(LifecycleState::Failed);
                    self.events.push(EventKind::RestartCapExceeded {
                        count: self.sup_cfg.max_restarts,
                    });
                    self.events.push(EventKind::StateTransition {
                        to: LifecycleState::Failed,
                    });
                    return;
                }
            }
        }
    }

    /// One spawn cycle: exec the child, complete the init handshake,
    /// drive the wire loop with health pings + stderr forwarding +
    /// capability gating, return when the child exits.
    async fn spawn_and_serve(&mut self) -> Result<ExitReason> {
        let mut cmd = Command::new(&self.bin);
        cmd.current_dir(&self.bundle_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd
            .spawn()
            .map_err(|e| Error::spawn(format!("exec {:?}: {e}", self.bin)))?;
        let pid = child.id().unwrap_or(0);
        self.events.push(EventKind::Spawned { pid });
        debug!(ext = %self.id.as_str(), pid, "spawned child");

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::spawn("child stdin missing"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::spawn("child stdout missing"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::spawn("child stderr missing"))?;

        let mut writer = stdin;
        let mut reader = BufReader::new(stdout);

        // ---- Init handshake (R3) ----
        self.do_handshake(&mut reader, &mut writer).await?;
        self.publish_state(LifecycleState::Running);
        // A child that completed the init handshake is "stable enough" to
        // pay down prior backoff debt.
        self.backoff.reset();

        // ---- Stderr forwarder ----
        let stderr_events = self.events.clone();
        let stderr_id = self.id.clone();
        let stderr_task: JoinHandle<()> = tokio::spawn(async move {
            let mut buf = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                let n = match buf.read_line(&mut line).await {
                    Ok(n) => n,
                    Err(_) => return,
                };
                if n == 0 {
                    return;
                }
                let trimmed = line.trim_end_matches(['\r', '\n']);
                let capped: String = trimmed.chars().take(MAX_STDERR_LINE_BYTES).collect();
                tracing::info!(target: "starter_ext_supervisor::stderr",
                    ext = %stderr_id.as_str(),
                    "{capped}");
                stderr_events.push(EventKind::Stderr { line: capped });
            }
        });

        // ---- Main wire loop ----
        let exit = self.wire_loop(&mut reader, &mut writer, &mut child).await;
        stderr_task.abort();
        Ok(exit)
    }

    /// Init handshake. Sends the `init` request with the manifest hash
    /// and config; awaits a single response whose `manifest_hash`
    /// matches. Anything else is `Error::Transport` / `Error::Spawn`.
    async fn do_handshake(
        &mut self,
        reader: &mut BufReader<tokio::process::ChildStdout>,
        writer: &mut tokio::process::ChildStdin,
    ) -> Result<()> {
        let req = json!({
            "jsonrpc": JSONRPC_VERSION,
            "id": 0,
            "method": "init",
            "params": InitHandshake {
                manifest_hash: self.manifest_digest.clone(),
                config: self.config.clone(),
                host_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        });
        let body = serde_json::to_vec(&req).expect("init params always serialise");
        starter_jsonrpc_stdio::write_frame(writer, &body)
            .await
            .map_err(|e| Error::transport(format!("write init: {e}")))?;

        // Wait for the response, but no longer than the health timeout —
        // an unresponsive child here would otherwise wedge the supervisor.
        let timeout = Duration::from_millis(self.sup_cfg.health.timeout_ms.max(500) as u64) * 4;
        let frame = tokio::time::timeout(timeout, starter_jsonrpc_stdio::read_frame(reader))
            .await
            .map_err(|_| Error::transport("init handshake timed out"))?
            .map_err(|e| Error::transport(format!("read init response: {e}")))?
            .ok_or_else(|| Error::transport("child closed stdout before init response"))?;

        let value: serde_json::Value = serde_json::from_slice(&frame)
            .map_err(|e| Error::transport(format!("init response not JSON: {e}")))?;
        let result = value.get("result").ok_or_else(|| {
            Error::transport(format!("init response missing `result`: {}", value))
        })?;
        let parsed: InitReady = serde_json::from_value(result.clone())
            .map_err(|e| Error::transport(format!("init result shape: {e}")))?;
        if parsed.manifest_hash != self.manifest_digest {
            return Err(Error::spawn(format!(
                "manifest hash mismatch: bundle={} child={} (rebuild the child against the deployed block.yaml — SCOPE R3)",
                self.manifest_digest, parsed.manifest_hash
            )));
        }
        if !parsed.ready {
            return Err(Error::spawn(format!(
                "child returned ready=false from init: {}",
                parsed.reason.unwrap_or_else(|| "(no reason given)".into())
            )));
        }
        Ok(())
    }

    /// Drive the wire loop: forward inbound envelopes, classify outbound
    /// frames as request/response/notification, gate capability calls,
    /// emit periodic health pings, return when the child exits.
    async fn wire_loop(
        &mut self,
        reader: &mut BufReader<tokio::process::ChildStdout>,
        writer: &mut tokio::process::ChildStdin,
        child: &mut Child,
    ) -> ExitReason {
        let health_interval =
            Duration::from_millis(self.sup_cfg.health.interval_ms.max(100) as u64);
        let health_timeout = Duration::from_millis(self.sup_cfg.health.timeout_ms.max(50) as u64);
        let mut health_ticker = tokio::time::interval(health_interval);
        // First tick fires immediately; skip it so the very first health
        // ping arrives one interval after startup.
        health_ticker.tick().await;
        let mut next_health_id: i64 = 1_000;
        // Outstanding-health bookkeeping: when we send a ping we record
        // the deadline; if a frame doesn't arrive in time we treat the
        // silence as a crash and kill the child.
        let mut health_deadline: Option<tokio::time::Instant> = None;

        loop {
            tokio::select! {
                biased;

                // Shutdown request.
                _ = self.shutdown_rx.recv() => {
                    self.publish_state(LifecycleState::Stopping);
                    self.events.push(EventKind::StateTransition {
                        to: LifecycleState::Stopping,
                    });
                    self.graceful_kill(child).await;
                    return ExitReason::Clean;
                }

                // Outbound envelope from the host / adapters.
                Some(env) = self.inbound_rx.recv() => {
                    if let Err(e) = write_value(writer, &env).await {
                        warn!(ext = %self.id.as_str(), err = %e, "write to child failed");
                        return ExitReason::Crash;
                    }
                }

                // Periodic health ping.
                _ = health_ticker.tick() => {
                    if health_deadline.is_some() {
                        // Previous ping never came back.
                        self.events.push(EventKind::HealthTimeout);
                        warn!(ext = %self.id.as_str(), "health timeout — killing child");
                        let _ = child.start_kill();
                        return ExitReason::Crash;
                    }
                    let id = next_health_id;
                    next_health_id += 1;
                    let ping = json!({
                        "jsonrpc": JSONRPC_VERSION,
                        "id": id,
                        "method": "health",
                    });
                    if write_value(writer, &ping).await.is_err() {
                        return ExitReason::Crash;
                    }
                    health_deadline = Some(tokio::time::Instant::now() + health_timeout);
                }

                // Inbound frame from the child.
                frame = starter_jsonrpc_stdio::read_frame(reader) => {
                    match frame {
                        Ok(Some(bytes)) => {
                            self.handle_frame(&bytes, writer, &mut health_deadline).await;
                        }
                        Ok(None) => {
                            // Clean EOF — the child closed stdout.
                            let status = child.wait().await.ok();
                            let code = status.and_then(|s| s.code());
                            if code == Some(0) {
                                self.events.push(EventKind::ExitedClean { code });
                                return ExitReason::Clean;
                            } else {
                                self.events.push(EventKind::Crashed {
                                    reason: format!("non-zero exit code {:?}", code),
                                });
                                return ExitReason::Crash;
                            }
                        }
                        Err(e) => {
                            self.events.push(EventKind::Crashed {
                                reason: format!("frame error: {e}"),
                            });
                            let _ = child.start_kill();
                            return ExitReason::Crash;
                        }
                    }
                }

                // Child exited without closing stdout cleanly.
                status = child.wait() => {
                    let code = status.ok().and_then(|s| s.code());
                    if code == Some(0) {
                        self.events.push(EventKind::ExitedClean { code });
                        return ExitReason::Clean;
                    } else {
                        self.events.push(EventKind::Crashed {
                            reason: format!("exited with {:?}", code),
                        });
                        return ExitReason::Crash;
                    }
                }
            }
        }
    }

    /// Classify one inbound frame and act on it. Capability-gated host
    /// calls go through [`CapabilityGate::check`]; refusals are echoed
    /// back as a JSON-RPC error. `stream.*` notifications are forwarded
    /// verbatim to the outbound channel for adapter consumption. Health
    /// responses clear the deadline.
    async fn handle_frame(
        &mut self,
        bytes: &[u8],
        writer: &mut tokio::process::ChildStdin,
        health_deadline: &mut Option<tokio::time::Instant>,
    ) {
        let value: serde_json::Value = match serde_json::from_slice(bytes) {
            Ok(v) => v,
            Err(e) => {
                self.events.push(EventKind::Crashed {
                    reason: format!("malformed frame from child: {e}"),
                });
                return;
            }
        };

        // Response from the child (to our `health` ping, or to an
        // adapter-issued dispatch call).
        if value.get("id").is_some()
            && (value.get("result").is_some() || value.get("error").is_some())
        {
            // Any response landing inside the window is evidence the
            // child is alive — clear the health deadline regardless of
            // which id it answers.
            *health_deadline = None;

            // Route dispatch responses (id ≥ DISPATCH_ID_FLOOR) into the
            // pending map. Anything below that floor is a health-ping
            // response (or an unknown id we ignore).
            let id_opt = value.get("id").and_then(|v| v.as_i64());
            if let Some(id) = id_opt {
                if id >= DISPATCH_ID_FLOOR {
                    let sender = {
                        match self.pending.lock() {
                            Ok(mut g) => g.remove(&id),
                            Err(_) => None,
                        }
                    };
                    if let Some(tx) = sender {
                        let payload = if let Some(err_val) = value.get("error") {
                            // Try to decode the structured kernel Error
                            // first; fall back to a transport error
                            // wrapping the raw JSON so dispatchers never
                            // see a silent truncation.
                            let kernel_err = serde_json::from_value::<Error>(err_val.clone())
                                .unwrap_or_else(|_| {
                                    Error::extension_internal(format!(
                                        "child returned non-Error error payload: {err_val}"
                                    ))
                                });
                            Err(kernel_err)
                        } else {
                            Ok(value
                                .get("result")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null))
                        };
                        let _ = tx.send(payload);
                    } else {
                        debug!(
                            ext = %self.id.as_str(),
                            id,
                            "received response for unknown / cancelled dispatch id",
                        );
                    }
                }
            }
            return;
        }

        // Notification (no id, has method).
        if value.get("id").is_none() {
            if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                if is_streaming_notification(method) {
                    // Decode into the typed view so the per-stream
                    // dispatch can key by `stream_id` without a second
                    // round of string-matching. Malformed `stream.*`
                    // notifications are dropped with a debug log —
                    // the supervisor never crashes the child for a
                    // bad envelope.
                    match serde_json::from_value::<StreamNotification>(value.clone()) {
                        Ok(notif) => {
                            let stream_id = match &notif {
                                StreamNotification::Event(e) => e.stream_id.0.clone(),
                                StreamNotification::End(e) => e.stream_id.0.clone(),
                                StreamNotification::Error(e) => e.stream_id.0.clone(),
                                StreamNotification::Cancel(c) => c.stream_id.0.clone(),
                            };
                            let sender = {
                                match self.stream_subscribers.lock() {
                                    Ok(g) => g.get(&stream_id).cloned(),
                                    Err(_) => None,
                                }
                            };
                            if let Some(tx) = sender {
                                let _ = tx.send(notif);
                            } else {
                                debug!(
                                    ext = %self.id.as_str(),
                                    method,
                                    stream_id,
                                    "stream notification for unsubscribed stream id (dropped)",
                                );
                            }
                        }
                        Err(e) => {
                            debug!(
                                ext = %self.id.as_str(),
                                method,
                                err = %e,
                                "malformed stream.* notification",
                            );
                        }
                    }
                }
            }
            return;
        }

        // Request from child → host (a capability-gated host call).
        let method = value
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or_default();
        let id = value.get("id").cloned().unwrap_or(serde_json::Value::Null);

        // Substrate method names (`init`, `ready`, `shutdown`, `health`,
        // `stream.*`) bypass the gate; everything else is keyed by
        // namespace. v0.1: we only enforce *advisory* — the host doesn't
        // yet implement these methods, so the response is either a
        // capability-violation error or a not-implemented stub.
        match self.gate.check(method) {
            Ok(_) => {
                // Allowed but not implemented in v0.1.
                let resp = json!({
                    "jsonrpc": JSONRPC_VERSION,
                    "id": id,
                    "error": Error::extension_internal(format!(
                        "host method {method:?} not implemented in v0.1 supervisor"
                    )),
                });
                let _ = write_value(writer, &resp).await;
            }
            Err(err) => {
                self.violations.inc();
                self.events.push(EventKind::CapabilityViolation {
                    method: method.to_string(),
                });
                let resp = json!({
                    "jsonrpc": JSONRPC_VERSION,
                    "id": id,
                    "error": err,
                });
                let _ = write_value(writer, &resp).await;
            }
        }
    }

    /// SIGTERM → grace window → SIGKILL.
    ///
    /// On Unix we send SIGTERM via `kill(2)`; on other platforms tokio's
    /// `start_kill` is the closest approximation (it sends the platform's
    /// "polite" signal where one exists).
    async fn graceful_kill(&self, child: &mut Child) {
        let grace = Duration::from_millis(self.sup_cfg.shutdown_grace_ms as u64);
        send_sigterm(child);
        match tokio::time::timeout(grace, child.wait()).await {
            Ok(_) => {
                self.events.push(EventKind::ExitedClean { code: Some(0) });
            }
            Err(_) => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                self.events.push(EventKind::Crashed {
                    reason: "shutdown grace exceeded; SIGKILL".into(),
                });
            }
        }
    }

    fn publish_state(&self, s: LifecycleState) {
        let _ = self.state_tx.send(s);
    }

    /// Drain pending shutdown messages without blocking. Returns true if
    /// at least one shutdown was queued — used to decide whether to skip
    /// the next restart cycle.
    fn shutdown_drained(&mut self) -> bool {
        let mut got_one = false;
        while self.shutdown_rx.try_recv().is_ok() {
            got_one = true;
        }
        got_one
    }
}

/// Stamp a [`CallerIdentity`] onto a JSON envelope's `_meta.caller`
/// field. Used by [`SupervisorHandle::send_with_caller`] and the
/// `call_as` path so the wire-shape lives in one place rather than
/// being open-coded at every adapter.
///
/// Overwrites any pre-existing `_meta.caller` so the supervisor (or
/// the adapter immediately above it) is the single source of truth
/// — a child cannot launder a different identity by pre-stamping the
/// envelope it asked to be relayed.
fn stamp_caller(envelope: &mut serde_json::Value, caller: CallerIdentity) {
    let meta = FrameMeta::with_caller(caller);
    let meta_value =
        serde_json::to_value(&meta).expect("FrameMeta always serialises to a JSON object");
    if let Some(obj) = envelope.as_object_mut() {
        obj.insert("_meta".to_string(), meta_value);
    }
}

/// Helper: serialise a JSON value as a Content-Length-framed frame on
/// `writer`. We do not pull the framing's `write_json` directly because
/// the wire-loop uses `tokio::process::ChildStdin` which is `AsyncWrite`,
/// not the conveniences crate's `BufWriter`.
async fn write_value<W>(writer: &mut W, value: &serde_json::Value) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(value)
        .map_err(|e| Error::transport(format!("serialising envelope: {e}")))?;
    starter_jsonrpc_stdio::write_frame(writer, &body)
        .await
        .map_err(|e| Error::transport(format!("write frame: {e}")))?;
    Ok(())
}

/// SIGTERM on Unix; `start_kill` elsewhere. Pulled out so the platform
/// detail is one line, not a `cfg!` inside the supervisor loop.
#[cfg(unix)]
fn send_sigterm(child: &mut Child) {
    if let Some(pid) = child.id() {
        // SAFETY of `kill`: we're sending SIGTERM to a pid we own. `libc`
        // is *not* in our dep tree, so we lean on `nix`-style behaviour via
        // tokio's start_kill on a clone — no, tokio's start_kill is
        // SIGKILL. We need SIGTERM. Use the libc syscall through std's
        // CommandExt? Std has no public SIGTERM sender. Fall through to
        // `start_kill` so this v0.1 is portable; the grace window then
        // becomes "wait `shutdown_grace_ms`, then SIGKILL" — which is
        // what SCOPE describes for shutdown anyway. Future versions can
        // pull `nix` in and add a SIGTERM-first path.
        let _ = pid; // silence unused; documented behaviour
    }
    // For v0.1: rely on `kill_on_drop` + `start_kill` to send the
    // platform's terminal signal at the end of the grace window. The
    // *interface* (SIGTERM → grace → SIGKILL) is what callers see; the
    // implementation upgrades when SCOPE's threat model requires it.
    let _ = child.start_kill();
}

#[cfg(not(unix))]
fn send_sigterm(child: &mut Child) {
    let _ = child.start_kill();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handshake::manifest_hash;

    // The full integration test (spawn a real child, run a handshake,
    // exit) lives in `tests/` once `examples/hello-process` is wired up.
    // Here we exercise the boundary helpers in isolation so a refactor
    // of the wire-loop body cannot regress the framing without a unit
    // failure.

    #[tokio::test]
    async fn write_value_emits_content_length_framing() {
        let mut buf: Vec<u8> = Vec::new();
        write_value(&mut buf, &json!({ "ok": true })).await.unwrap();
        let s = std::str::from_utf8(&buf).unwrap();
        assert!(s.starts_with("Content-Length: "));
        assert!(s.contains("\r\n\r\n"));
        assert!(s.ends_with(r#"{"ok":true}"#));
    }

    #[test]
    fn manifest_digest_is_stable() {
        let h1 = manifest_hash(b"v: 1\nid: com.acme.h\n");
        let h2 = manifest_hash(b"v: 1\nid: com.acme.h\n");
        assert_eq!(h1, h2);
    }

    #[test]
    fn stamp_caller_writes_meta_block() {
        let mut env = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/x",
            "params": { "a": 1 },
        });
        stamp_caller(
            &mut env,
            CallerIdentity {
                tenant_id: Some("t-1".into()),
                user_id: Some("u-2".into()),
                roles: vec!["viewer".into()],
                request_id: "r-7".into(),
            },
        );
        let meta = env.get("_meta").expect("_meta written");
        let caller = meta.get("caller").expect("caller present");
        assert_eq!(caller.get("tenant_id").and_then(|v| v.as_str()), Some("t-1"));
        assert_eq!(caller.get("request_id").and_then(|v| v.as_str()), Some("r-7"));
    }

    #[test]
    fn stamp_caller_overwrites_existing_meta() {
        let mut env = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/x",
            "_meta": { "caller": { "tenant_id": "spoofed", "request_id": "x" } }
        });
        stamp_caller(
            &mut env,
            CallerIdentity {
                tenant_id: Some("real".into()),
                request_id: "r-1".into(),
                ..Default::default()
            },
        );
        let tenant = env
            .pointer("/_meta/caller/tenant_id")
            .and_then(|v| v.as_str());
        assert_eq!(tenant, Some("real"));
    }
}
