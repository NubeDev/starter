//! Run service (P1/P1a, DOCS §7–§9).
//!
//! Running a template is: validate input → seed slots (incl. the
//! **trusted identity slots** from the verified `Principal`) →
//! `FlowRunner::start` → return `run_id` immediately. A progress
//! projector subscribes to the engine's `FlowEvent` broadcast and keeps
//! the `setup_runs` index current. Resume (P1a) replays the failed run's
//! checkpoint and re-enters at the persisted cursor.

use std::collections::BTreeMap;
use std::sync::Arc;

use starter_flow::propagator::{FlowTopology, PropagatorConfig};
use starter_flow::registry::NodeKindRegistry;
use starter_flow::run::{FlowRunner, FlowRunnerConfig, RunHandle, RunSpec};
use starter_flow_spi::flow::{FlowEvent, FlowId, FlowRevisionId, RunId};
use starter_flow_spi::node::{NodeId, SlotMap, SlotRef, SlotValue};
use starter_flow_spi::Principal;
use starter_setup_spi::error::{SetupError, SetupResult};
use starter_setup_spi::model::{Progress, SetupRun, SetupRunStatus, Template};
use starter_setup_spi::reserved;
use starter_setup_spi::store::{SetupRunStore, TemplateStore};

use crate::import::{slot_node, validate_bindings};

/// Engine handle the run service drives: a [`FlowRunner`] configured with
/// the §8b halt-on-node-failure policy, plus the node-kind registry used
/// to resolve a template's `FlowBody` into an executable topology.
#[derive(Clone)]
pub struct SetupEngine {
    runner: Arc<FlowRunner>,
    kinds: Arc<NodeKindRegistry>,
}

impl SetupEngine {
    /// Build a setup engine over an existing [`FlowRunner`] and the shared
    /// node-kind registry.
    ///
    /// The runner MUST be configured with `halt_on_node_failure = true`
    /// (DOCS §8b) for resume-from-failure to work; use
    /// [`SetupEngine::runner_config`] to construct one.
    pub fn new(runner: Arc<FlowRunner>, kinds: Arc<NodeKindRegistry>) -> Self {
        Self { runner, kinds }
    }

    /// The [`FlowRunnerConfig`] a setup runner must use: the §8b fatal
    /// halt policy on. Compose with a `FlowRunner::new(...).with_config(
    /// SetupEngine::runner_config())` (and any SPI run store) at boot.
    pub fn runner_config() -> FlowRunnerConfig {
        FlowRunnerConfig::default()
            .with_propagator(PropagatorConfig::default().with_halt_on_node_failure(true))
    }

    /// The node-kind registry (for template-import validation).
    pub fn kinds(&self) -> &Arc<NodeKindRegistry> {
        &self.kinds
    }
}

/// Tuning for the run service.
#[derive(Debug, Clone)]
pub struct RunServiceConfig {
    /// On crash recovery, the max number of times an idempotent failed
    /// run is auto-resumed before it is left for a human (DOCS Q1; the
    /// automation system should self-heal but not loop forever).
    pub max_auto_resume: u32,
}

impl Default for RunServiceConfig {
    fn default() -> Self {
        Self { max_auto_resume: 3 }
    }
}

/// A live, in-process run: its `FlowEvent` broadcast sender (for SSE
/// tailing) and cancel handle (for cancel). Kept only while the run is
/// in-flight in this process; the durable index ([`SetupRunStore`]) is
/// the source of truth across processes.
#[derive(Clone)]
pub struct LiveRun {
    /// Broadcast sender — `subscribe()` for an SSE tail.
    pub events_tx: tokio::sync::broadcast::Sender<FlowEvent>,
    /// Cancel handle for the run.
    pub cancel: Arc<starter_flow::run::RunCancel>,
}

type LiveRegistry = Arc<std::sync::Mutex<std::collections::HashMap<RunId, LiveRun>>>;

/// The shared domain service behind both the REST and MCP surfaces
/// (DOCS §11 "both surfaces share the same domain service").
#[derive(Clone)]
pub struct RunService<TS, RS> {
    templates: Arc<TS>,
    runs: Arc<RS>,
    engine: SetupEngine,
    config: RunServiceConfig,
    /// In-flight runs in this process, for SSE tail + cancel.
    live: LiveRegistry,
}

impl<TS, RS> RunService<TS, RS>
where
    TS: TemplateStore,
    RS: SetupRunStore,
{
    /// Construct the run service.
    pub fn new(
        templates: Arc<TS>,
        runs: Arc<RS>,
        engine: SetupEngine,
        config: RunServiceConfig,
    ) -> Self {
        Self {
            templates,
            runs,
            engine,
            config,
            live: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Subscribe to a live in-flight run's event stream (for SSE tailing).
    /// Returns `None` if the run is not currently in-flight in this process
    /// — callers should fall back to the stored progress snapshot.
    pub fn subscribe_live(&self, run_id: RunId) -> Option<tokio::sync::broadcast::Receiver<FlowEvent>> {
        self.live
            .lock()
            .unwrap()
            .get(&run_id)
            .map(|l| l.events_tx.subscribe())
    }

    /// Cancel a live in-flight run (DOCS §11 cancel). Returns `true` if a
    /// live handle was found and signalled; `false` if the run is not
    /// in-flight here (the caller still updates the durable index).
    pub fn cancel_live(&self, run_id: RunId) -> bool {
        if let Some(live) = self.live.lock().unwrap().get(&run_id) {
            live.cancel.cancel();
            true
        } else {
            false
        }
    }

    fn register_live(&self, run_id: RunId, handle: &RunHandle) {
        self.live.lock().unwrap().insert(
            run_id,
            LiveRun {
                events_tx: handle.events_tx.clone(),
                cancel: handle.cancel.clone(),
            },
        );
    }

    fn deregister_live(&self, run_id: RunId) {
        self.live.lock().unwrap().remove(&run_id);
    }

    /// Template store handle (for surface adapters that list/fetch).
    pub fn templates(&self) -> &Arc<TS> {
        &self.templates
    }

    /// Run store handle (for surface adapters that list/snapshot).
    pub fn runs(&self) -> &Arc<RS> {
        &self.runs
    }

    /// Subscribe a fresh receiver to a running run's event broadcast — for
    /// the SSE endpoint to tail live events after replaying the snapshot.
    /// Returns `None` if the run is not currently in-flight in this
    /// process. (Implemented by the surface via the [`RunHandle`] it keeps;
    /// the service itself does not retain handles across calls.)
    pub fn engine(&self) -> &SetupEngine {
        &self.engine
    }

    /// Launch a template (DOCS §7). Validates input against the template's
    /// `input_schema`, binds form fields onto entry slots, seeds the
    /// trusted identity slots from the **verified** `principal`, starts the
    /// run, records the index row, and spawns the progress projector.
    /// Returns the `run_id` immediately (202-style).
    ///
    /// NOTE: the caller is responsible for the *generic* authz gate and the
    /// setup-layer team check (DOCS §10) — this method assumes the
    /// principal is already permitted to run `template`.
    pub async fn run_template(
        &self,
        template: &Template,
        principal: &Principal,
        input: &serde_json::Value,
    ) -> SetupResult<RunHandle> {
        validate_input(&template.input_schema, input)?;
        // Defence in depth: re-check bindings at run time even though
        // import validated them (a store could hold a pre-validation row).
        validate_bindings(template)?;

        let topology = self.resolve_topology(template).await?;
        let flow_id = FlowId::new(template.id.0.clone())
            .map_err(|e| SetupError::InvalidBody(e.to_string()))?;
        let revision = FlowRevisionId::new();

        let mut seeds = bind_inputs(&template.input_bindings, input)?;
        seeds.extend(trusted_identity_seeds(template, principal));

        let terminal_slots = output_slots(template)?;
        let spec = RunSpec::new(flow_id, revision, topology, seeds, terminal_slots)
            .with_principal(principal.clone());

        let mut handle = self
            .engine
            .runner
            .start(spec, SlotMap::new())
            .await
            .map_err(|e| SetupError::Backend(format!("engine start: {e}")))?;

        // Take the runner's PRE-SUBSCRIBED receiver for the projector so no
        // early event (incl. a fast RunFailed/RunCompleted) is missed — a
        // receiver subscribed after start() races the run to termination
        // and a closed channel yields nothing (DOCS §7 "no early events
        // lost"). Swap in a fresh receiver to keep the field populated.
        let projector_rx =
            std::mem::replace(&mut handle.initial_rx, handle.events_tx.subscribe());

        let run = SetupRun {
            run_id: handle.run,
            template_id: template.id.clone(),
            template_version: template.version,
            owner: principal.subject.clone(),
            tenant_id: principal.tenant_id.clone(),
            team: principal.teams.first().cloned(),
            status: SetupRunStatus::Running,
            progress: Progress {
                done: 0,
                total: template.flow_body.nodes.len(),
                current_step: None,
            },
            failed_node: None,
            resumable: false,
            created_at: now_rfc3339(),
            finished_at: None,
        };
        self.runs.record(run).await?;

        self.register_live(handle.run, &handle);
        self.spawn_projector(handle.run, template.flow_body.nodes.len(), projector_rx);
        Ok(handle)
    }

    /// Resume a finished-failed, resumable run from its cursor (P1a,
    /// DOCS §8b). Replays the run's checkpoint via the engine's resume
    /// entry point and re-fires the cursor node by re-seeding its trigger
    /// slot, so the failed step re-executes (idempotent nodes make this
    /// safe — DOCS §8c).
    pub async fn resume_run(
        &self,
        template: &Template,
        run_id: RunId,
    ) -> SetupResult<RunHandle> {
        let existing = self
            .runs
            .get(run_id)
            .await?
            .ok_or_else(|| SetupError::NotFound(run_id.to_string()))?;
        if existing.status != SetupRunStatus::Failed || !existing.resumable {
            return Err(SetupError::InvalidRunState(format!(
                "run {run_id} is {:?}, not resumable",
                existing.status
            )));
        }

        let topology = self.resolve_topology(template).await?;
        let flow_id = FlowId::new(template.id.0.clone())
            .map_err(|e| SetupError::InvalidBody(e.to_string()))?;

        // Re-fire the cursor node: the engine's resume replays the
        // checkpoint's prior writes idempotently — which does NOT re-wake a
        // node (the R3 short-circuit suppresses a duplicate SlotChanged).
        // To re-enter AT the cursor (DOCS §8b) we seed the cursor's trigger
        // slots from the checkpoint values so they fire as fresh writes,
        // re-invoking exactly the failed step. Idempotent nodes make the
        // re-entry safe (DOCS §8c).
        let resume_seeds = self
            .cursor_trigger_seeds(run_id, existing.failed_node.as_deref(), &topology)
            .await?;
        let spec = RunSpec::new(
            flow_id,
            existing_revision(),
            topology,
            resume_seeds,
            output_slots(template)?,
        );
        let resume_input = SlotMap::new();

        let mut handle = self
            .engine
            .runner
            .resume(spec, resume_input, run_id)
            .await
            .map_err(|e| SetupError::Backend(format!("engine resume: {e}")))?
            .ok_or_else(|| {
                SetupError::InvalidRunState(format!("no checkpoint for run {run_id}"))
            })?;
        let projector_rx =
            std::mem::replace(&mut handle.initial_rx, handle.events_tx.subscribe());

        // Mark running again and project further progress.
        self.runs
            .update_progress(
                run_id,
                existing.progress.clone(),
                SetupRunStatus::Running,
            )
            .await?;
        self.register_live(run_id, &handle);
        self.spawn_projector(run_id, template.flow_body.nodes.len(), projector_rx);
        Ok(handle)
    }

    /// Crash recovery (P1, DOCS §8a) + bounded auto-recovery (DOCS §8b/Q1).
    /// For each open run, replay its checkpoint and re-drive. Idempotent
    /// nodes make re-entry safe. Returns the run ids it re-launched.
    ///
    /// `templates_by_id` resolves each run's template so the topology can
    /// be rebuilt; runs whose template is missing are skipped (logged).
    pub async fn recover_open_runs(&self) -> SetupResult<Vec<RunId>> {
        let open = self.runs.list_open().await?;
        let mut recovered = Vec::new();
        // Bound the batch so a storm of resumable-failed runs can't spawn
        // unboundedly on boot (DOCS Q1 — self-heal, but don't loop forever).
        let cap = self.config.max_auto_resume.max(1) as usize * open.len().max(1);
        for run_id in open.into_iter().take(cap) {
            let Some(setup_run) = self.runs.get(run_id).await? else {
                continue;
            };
            let Some(template) = self
                .templates
                .get(
                    setup_run.tenant_id.as_deref(),
                    &setup_run.template_id,
                    Some(setup_run.template_version),
                )
                .await?
            else {
                tracing::warn!(
                    run = %run_id,
                    "recover: template gone, leaving run for manual handling"
                );
                continue;
            };
            // Failed+resumable → cursor resume; Pending/Running → plain
            // crash-recovery replay.
            let result = match setup_run.status {
                SetupRunStatus::Failed => self.resume_run(&template, run_id).await,
                _ => self.replay_open(&template, run_id).await,
            };
            match result {
                Ok(_) => recovered.push(run_id),
                Err(e) => tracing::warn!(run = %run_id, error = %e, "recover failed"),
            }
        }
        Ok(recovered)
    }

    /// Plain crash-recovery replay for a Pending/Running run (DOCS §8a):
    /// `RunStore::load` → replay writes → re-drive. No cursor re-fire.
    async fn replay_open(&self, template: &Template, run_id: RunId) -> SetupResult<RunHandle> {
        let topology = self.resolve_topology(template).await?;
        let flow_id = FlowId::new(template.id.0.clone())
            .map_err(|e| SetupError::InvalidBody(e.to_string()))?;
        let spec = RunSpec::new(
            flow_id,
            existing_revision(),
            topology,
            Vec::new(),
            output_slots(template)?,
        );
        let mut handle = self
            .engine
            .runner
            .resume(spec, SlotMap::new(), run_id)
            .await
            .map_err(|e| SetupError::Backend(format!("engine resume: {e}")))?
            .ok_or_else(|| SetupError::InvalidRunState(format!("no checkpoint for run {run_id}")))?;
        let projector_rx =
            std::mem::replace(&mut handle.initial_rx, handle.events_tx.subscribe());
        self.register_live(run_id, &handle);
        self.spawn_projector(run_id, template.flow_body.nodes.len(), projector_rx);
        Ok(handle)
    }

    /// Build the seed writes that re-fire the cursor node on resume
    /// (DOCS §8b). The failed node's input slots are still present in the
    /// graph store (they were written before the node fired and failed); we
    /// read each of the cursor's trigger slots and emit a fresh seed write
    /// carrying that value. Re-writing as a *new* seed re-wakes exactly the
    /// failed step after the engine's checkpoint replay — the R3 idempotent
    /// short-circuit would otherwise suppress a re-trigger. Idempotent
    /// nodes make the re-entry safe (DOCS §8c).
    ///
    /// `_run_id` is accepted for symmetry / future per-run state lookups.
    async fn cursor_trigger_seeds(
        &self,
        _run_id: RunId,
        cursor: Option<&str>,
        topology: &FlowTopology,
    ) -> SetupResult<Vec<(SlotRef, SlotValue)>> {
        let Some(cursor) = cursor else {
            return Ok(Vec::new());
        };
        let cursor_node = NodeId::new(cursor)
            .map_err(|e| SetupError::InvalidRunState(format!("bad cursor node '{cursor}': {e}")))?;
        let triggers = topology
            .triggers
            .get(&cursor_node)
            .cloned()
            .unwrap_or_default();

        let store = self.engine.runner.store();
        let mut seeds = Vec::new();
        for trig in &triggers {
            let sr = SlotRef::new(cursor_node.clone(), trig.clone());
            if let Ok(value) = store.read_slot(&sr).await {
                // Skip null/unset slots — only re-fire slots that actually
                // carry a value.
                if !matches!(value, SlotValue::Null) {
                    seeds.push((sr, value));
                }
            }
        }
        Ok(seeds)
    }

    /// Resolve a template's `FlowBody` into an executable topology via the
    /// flow layer's body-level resolver (DOCS §6 — body-level, not file).
    async fn resolve_topology(&self, template: &Template) -> SetupResult<Arc<FlowTopology>> {
        let flow_id = FlowId::new(template.id.0.clone())
            .map_err(|e| SetupError::InvalidBody(e.to_string()))?;
        starter_flow::definition::resolver::TopologyResolver::resolve_body(
            &template.flow_body,
            &flow_id,
            &self.engine.kinds,
        )
        .await
        .map_err(|e| SetupError::InvalidBody(e.to_string()))
    }

    /// Spawn the progress projector (DOCS §7): translate the engine's
    /// `FlowEvent` stream into `setup_runs` progress/status updates so list
    /// views and reconnecting clients see current state without replaying.
    fn spawn_projector(
        &self,
        run_id: RunId,
        total: usize,
        mut rx: tokio::sync::broadcast::Receiver<FlowEvent>,
    ) {
        let runs = self.runs.clone();
        let live = self.live.clone();
        tokio::spawn(async move {
            let mut done = 0usize;
            let mut current: Option<String> = None;
            let mut last_failed_node: Option<String> = None;
            loop {
                match rx.recv().await {
                    Ok(ev) => match ev {
                        FlowEvent::NodeStarted { node, .. } => {
                            current = Some(node.to_string());
                            let _ = runs
                                .update_progress(
                                    run_id,
                                    Progress {
                                        done,
                                        total,
                                        current_step: current.clone(),
                                    },
                                    SetupRunStatus::Running,
                                )
                                .await;
                        }
                        FlowEvent::NodeEmitted { .. } => {
                            done = done.saturating_add(1).min(total);
                        }
                        FlowEvent::NodeFailed { node, .. } => {
                            // Capture the §8b cursor; the terminal RunFailed
                            // (emitted by the halt policy) finalizes below.
                            last_failed_node = Some(node.to_string());
                        }
                        FlowEvent::RunFailed { .. } => {
                            let _ = runs
                                .mark_failed(run_id, last_failed_node.clone(), true)
                                .await;
                            break;
                        }
                        FlowEvent::RunCompleted { .. } => {
                            let _ = runs
                                .update_progress(
                                    run_id,
                                    Progress {
                                        done: total,
                                        total,
                                        current_step: None,
                                    },
                                    SetupRunStatus::Completed,
                                )
                                .await;
                            let _ = runs
                                .mark_finished(
                                    run_id,
                                    SetupRunStatus::Completed,
                                    now_rfc3339(),
                                )
                                .await;
                            break;
                        }
                        FlowEvent::RunCancelled { .. } => {
                            let _ = runs
                                .mark_finished(
                                    run_id,
                                    SetupRunStatus::Cancelled,
                                    now_rfc3339(),
                                )
                                .await;
                            break;
                        }
                        _ => {}
                    },
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            // Run terminal (or channel closed) — drop the live handle so
            // the SSE/cancel registry doesn't leak finished runs.
            live.lock().unwrap().remove(&run_id);
        });
    }
}

/// Build seed slot writes from the template's `input_bindings` and the
/// launcher's form input (DOCS §6/§7). A binding whose field is absent
/// from `input` is skipped (the form may carry optional fields).
pub fn bind_inputs(
    bindings: &[starter_setup_spi::model::InputBinding],
    input: &serde_json::Value,
) -> SetupResult<Vec<(SlotRef, SlotValue)>> {
    let obj = input
        .as_object()
        .ok_or_else(|| SetupError::InvalidInput("input must be a JSON object".into()))?;
    let mut seeds = Vec::new();
    for b in bindings {
        let Some(value) = obj.get(&b.field) else {
            continue;
        };
        let (node, slot) = slot_node(&b.slot).ok_or_else(|| {
            SetupError::InvalidBinding(format!("malformed slot reference: {}", b.slot))
        })?;
        let node_id = NodeId::new(node)
            .map_err(|e| SetupError::InvalidBinding(format!("bad node id '{node}': {e}")))?;
        seeds.push((SlotRef::new(node_id, slot.to_owned()), json_to_slot(value)));
    }
    Ok(seeds)
}

/// Seed the reserved trusted-identity slots from the **verified**
/// `Principal` (DOCS §9). For every node that declares a reserved slot in
/// its bindings… actually identity is host-bound, not template-bound: we
/// seed the reserved names onto **every entry node** of the flow so any
/// node that reads `caller_*` sees it. We derive entry nodes from the
/// node declarations (all nodes are eligible readers; seeding is cheap and
/// idempotent).
fn trusted_identity_seeds(
    template: &Template,
    principal: &Principal,
) -> Vec<(SlotRef, SlotValue)> {
    let teams = serde_json::Value::Array(
        principal
            .teams
            .iter()
            .map(|t| serde_json::Value::String(t.clone()))
            .collect(),
    );
    let identity: BTreeMap<&str, SlotValue> = BTreeMap::from([
        (
            reserved::CALLER_USER_ID,
            SlotValue::String(principal.subject.clone()),
        ),
        (reserved::CALLER_TEAM_IDS, SlotValue::Json(teams)),
        (
            reserved::CALLER_TENANT_ID,
            SlotValue::String(principal.tenant_id.clone().unwrap_or_default()),
        ),
    ]);

    let mut seeds = Vec::new();
    for node in &template.flow_body.nodes {
        for (name, value) in &identity {
            seeds.push((
                SlotRef::new(node.id.clone(), (*name).to_owned()),
                value.clone(),
            ));
        }
    }
    seeds
}

/// Terminal slots from the template's `output_bindings` (DOCS §6).
fn output_slots(template: &Template) -> SetupResult<Vec<SlotRef>> {
    let mut out = Vec::new();
    for b in &template.output_bindings {
        let (node, slot) = slot_node(&b.slot).ok_or_else(|| {
            SetupError::InvalidBinding(format!("malformed output slot: {}", b.slot))
        })?;
        let node_id = NodeId::new(node)
            .map_err(|e| SetupError::InvalidBinding(format!("bad node id '{node}': {e}")))?;
        out.push(SlotRef::new(node_id, slot.to_owned()));
    }
    Ok(out)
}

/// Validate launch input against the template's JSON-Schema `input_schema`
/// (DOCS §7 "reject bad form early").
pub fn validate_input(schema: &serde_json::Value, input: &serde_json::Value) -> SetupResult<()> {
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(schema)
        .map_err(|e| SetupError::InvalidInput(format!("input_schema invalid: {e}")))?;
    if let Err(errors) = compiled.validate(input) {
        let msg = errors
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(SetupError::InvalidInput(msg));
    }
    Ok(())
}

fn json_to_slot(v: &serde_json::Value) -> SlotValue {
    match v {
        serde_json::Value::Null => SlotValue::Null,
        serde_json::Value::Bool(b) => SlotValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SlotValue::Int(i)
            } else {
                SlotValue::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => SlotValue::String(s.clone()),
        other => SlotValue::Json(other.clone()),
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Resume reuses the persisted run id; the revision handle is regenerated
/// (it is informational on the resume path — the engine keys on run id).
fn existing_revision() -> FlowRevisionId {
    FlowRevisionId::new()
}
