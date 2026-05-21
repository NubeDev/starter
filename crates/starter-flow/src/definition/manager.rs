//! `DefinitionManager` — the HR1 publish chokepoint.
//!
//! Per `DOCS/flow/scope/hot-reload.md` HR1: every definition edit —
//! REST handler, CLI command, UI canvas save, host-dir file-watch,
//! extension reload, programmatic API — funnels through
//! [`DefinitionManager::publish`]. This module owns that one function
//! and the support types it produces.
//!
//! Phase HR-1 ships the chokepoint with:
//!
//! 1. Body parsing into the typed [`FlowBody`] shape.
//! 2. Per-node `validate_settings` via the kind's schema.
//! 3. Full [`TopologyResolver::resolve_body`] dry-run so the publish
//!    refuses anything that wouldn't mount.
//! 4. JCS canonicalisation + `blake3` hash for the idempotent
//!    short-circuit (HR1 step 3).
//! 5. Atomic write through [`FlowStore::put`] (HR1 step 4).
//! 6. [`FlowDefinitionEvent`] emission on the engine's definition
//!    bus.
//!
//! `ActiveTopology` swap (HR2), the diff classifier (HR2), per-flow
//! `apply_policy` dispatch (HR4), and the full observability surface
//! (HR3) layer on top in later phases without changing this
//! chokepoint's interface.

use std::sync::Arc;

use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{Instrument, debug, info, info_span, warn};

use starter_flow_spi::definition::{
    ApplyPolicy, DefinitionSource, EditKindTag, FlowDefinitionEvent,
};
use starter_flow_spi::flow::{FlowError, FlowId, FlowRevision, FlowRevisionId, FlowStore};
use starter_flow_spi::graph::{GraphError, GraphStore, WriteSlotOpts};

use crate::definition::active::ActiveTopologies;
use crate::definition::body::{self, FlowBody};
use crate::definition::canonical::{body_hash, BodyHash};
use crate::definition::classifier::{classify, EditKind};
use crate::definition::metrics::DefinitionMetricsCell;
use crate::definition::resolver::{TopologyResolver, TopologyResolverError};
use crate::definition::runs::{RunRegistration, RunRegistry};
use crate::registry::NodeKindRegistry;
use crate::run::RunCancel;

/// Default broadcast capacity for the definition bus.
///
/// Mirrors the `RunOpts::event_broadcast_capacity` shape from
/// `starter-flow-spi::flow` — sized so a slow consumer (a UI canvas
/// over a flaky network) can drop messages without back-pressuring
/// the publish call. The propagator's own broadcast also defaults
/// to 1024.
pub const DEFAULT_DEFINITION_BUS_CAPACITY: usize = 1024;

/// Outcome of a successful [`DefinitionManager::publish`] call.
///
/// Two terminal happy paths:
///
/// - [`Self::Published`] — the draft was a new revision; a fresh
///   [`FlowRevisionId`] is now durable in [`FlowStore`].
/// - [`Self::ShortCircuited`] — HR1's idempotent short-circuit hit;
///   the head's hash matched the draft's hash; no new revision was
///   written.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PublishOutcome {
    /// A new revision was written.
    Published {
        /// The newly-written revision id.
        revision: FlowRevisionId,
        /// The previous head, if any.
        prev_head: Option<FlowRevisionId>,
        /// Classifier output. Phase HR-1 always emits
        /// [`EditKindTag::Initial`] (first publish) or
        /// [`EditKindTag::Structural`] (every other publish) — the
        /// pure diff classifier lands HR-2.
        kind: EditKindTag,
    },
    /// The draft was identical to the current head; no new revision.
    ShortCircuited {
        /// The current head the draft collapsed onto.
        head: FlowRevisionId,
    },
}

/// Errors returned by [`DefinitionManager::publish`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PublishError {
    /// The draft body was syntactically invalid, referenced an
    /// unknown kind, or otherwise failed
    /// [`TopologyResolver::resolve_body`].
    #[error("resolve failed: {0}")]
    Resolve(#[from] TopologyResolverError),

    /// [`FlowStore`] rejected the write (backend unavailable,
    /// degraded, etc.). The previous head is unchanged.
    #[error("flow-store write failed: {0}")]
    Store(#[from] FlowError),

    /// A settings-path slot write failed against the live
    /// [`GraphStore`]. The new revision is durable in the
    /// [`FlowStore`] (the write that triggered this error happens
    /// AFTER the revision lands), and the active topology has been
    /// swapped if the edit was [`EditKind::Mixed`]; HR4's apply
    /// policy decides how the engine recovers.
    #[error("graph-store write failed: {0}")]
    Graph(#[from] GraphError),
}

/// Per-flow outcome counts from [`DefinitionManager::boot_resume`].
///
/// Used by the engine startup path to log a single line summarising
/// how many flows came up cleanly versus how many landed in
/// [`FlowDefinitionEvent::ResolveFailed`]. Tests assert on the
/// struct shape so the boot-walk contract is explicit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct BootResumeReport {
    /// Flows that resolved and were installed into
    /// [`ActiveTopologies`].
    pub mounted: usize,
    /// Flows whose head loaded but failed to resolve (e.g.
    /// references a kind that's not registered today). Each
    /// failure emits a [`FlowDefinitionEvent::ResolveFailed`].
    pub failed: usize,
    /// Flows whose [`FlowStore::head`] returned `None` (the flow
    /// row exists but has never been published) — nothing to
    /// mount.
    pub skipped: usize,
}

/// The HR1 publish chokepoint.
///
/// Owns the dependencies the chokepoint needs (the persistence seam
/// and the kind registry) and the broadcast bus consumers subscribe to
/// for [`FlowDefinitionEvent`] notifications. Constructed by the
/// host binary at engine wire-up time and stored on the `Engine`
/// (the wire-up is Phase HR-2 work; HR-1 ships the manager as a
/// standalone unit that's easy to test).
pub struct DefinitionManager {
    store: Arc<dyn FlowStore>,
    kinds: Arc<NodeKindRegistry>,
    graph: Option<Arc<dyn GraphStore>>,
    active: Arc<ActiveTopologies>,
    events: broadcast::Sender<FlowDefinitionEvent>,
    metrics: Arc<DefinitionMetricsCell>,
    runs: Arc<RunRegistry>,
    /// HR-6 graveyard of flows whose head currently fails to
    /// resolve. Populated by `publish` and `boot_resume` when
    /// the resolver errors and by `on_kind_deregistered` when a
    /// deregister revokes a live topology; drained by
    /// `on_kind_registered` when the missing kind shows up
    /// (HR8 first paragraph).
    failed: tokio::sync::RwLock<std::collections::HashMap<FlowId, FlowRevisionId>>,
}

impl DefinitionManager {
    /// Construct a manager with the default broadcast capacity and
    /// no attached graph store. Settings-only edits will land in
    /// the [`FlowStore`] and update the active topology but will
    /// NOT project onto live slots until [`Self::attach_graph`]
    /// (or the [`Self::with_graph`] constructor) gives the manager
    /// a [`GraphStore`] handle.
    pub fn new(store: Arc<dyn FlowStore>, kinds: Arc<NodeKindRegistry>) -> Self {
        Self::with_capacity(store, kinds, DEFAULT_DEFINITION_BUS_CAPACITY)
    }

    /// Construct a manager with a custom broadcast capacity.
    pub fn with_capacity(
        store: Arc<dyn FlowStore>,
        kinds: Arc<NodeKindRegistry>,
        capacity: usize,
    ) -> Self {
        let (events, _) = broadcast::channel(capacity.max(1));
        Self {
            store,
            kinds,
            graph: None,
            active: Arc::new(ActiveTopologies::new()),
            events,
            metrics: DefinitionMetricsCell::new(),
            runs: Arc::new(RunRegistry::new()),
            failed: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Convenience constructor: build a manager pre-wired to a
    /// [`GraphStore`] so the settings path can execute.
    pub fn with_graph(
        store: Arc<dyn FlowStore>,
        kinds: Arc<NodeKindRegistry>,
        graph: Arc<dyn GraphStore>,
    ) -> Self {
        let mut mgr = Self::new(store, kinds);
        mgr.graph = Some(graph);
        mgr
    }

    /// Attach (or replace) the live [`GraphStore`] used by the
    /// settings path. Returns the previous graph store, if any.
    pub fn attach_graph(&mut self, graph: Arc<dyn GraphStore>) -> Option<Arc<dyn GraphStore>> {
        self.graph.replace(graph)
    }

    /// Borrow the [`ActiveTopologies`] registry the manager swaps
    /// through. Engines hand this to the run path so per-step slot
    /// reads see the freshest topology.
    pub fn active_topologies(&self) -> Arc<ActiveTopologies> {
        self.active.clone()
    }

    /// Borrow the per-engine definition-layer counter cell.
    /// Hosts emitting Prometheus take a
    /// [`crate::definition::metrics::DefinitionMetrics`] snapshot
    /// via `metrics().snapshot()` and re-export under the names
    /// `DOCS/flow/scope/hot-reload.md` Observability lists.
    pub fn metrics(&self) -> Arc<DefinitionMetricsCell> {
        self.metrics.clone()
    }

    /// Borrow the [`RunRegistry`] this manager uses to drive
    /// HR-4 `Restart` cancellations. The per-run `FlowRunner`
    /// calls [`Self::register_run`] to enrol its [`RunCancel`]
    /// and keeps the returned [`RunRegistration`] alive for the
    /// run's lifetime; on a structural swap whose previous
    /// revision's `apply_policy` is `Restart` (or `LiveMigrate`
    /// falling back to `Restart`), the publish path calls
    /// [`RunRegistry::cancel_for`] for the prior revision.
    pub fn runs(&self) -> Arc<RunRegistry> {
        self.runs.clone()
    }

    /// Register an in-flight run's [`RunCancel`] handle under
    /// `(flow, revision)` so a future HR-4 `Restart` swap can
    /// fire it. The returned [`RunRegistration`] guard removes
    /// the entry when dropped — runners hold it for the run's
    /// lifetime.
    pub fn register_run(
        &self,
        flow: FlowId,
        revision: FlowRevisionId,
        cancel: Arc<RunCancel>,
    ) -> RunRegistration {
        self.runs.register(flow, revision, cancel)
    }

    /// Borrow the [`FlowStore`] this manager writes through.
    pub fn store(&self) -> &Arc<dyn FlowStore> {
        &self.store
    }

    /// Borrow the [`NodeKindRegistry`] this manager resolves
    /// against.
    pub fn kinds(&self) -> &Arc<NodeKindRegistry> {
        &self.kinds
    }

    /// Subscribe to [`FlowDefinitionEvent`]s emitted by this
    /// manager. Each subscriber gets its own receiver; messages are
    /// dropped on slow subscribers per the `broadcast` channel
    /// contract.
    pub fn subscribe(&self) -> broadcast::Receiver<FlowDefinitionEvent> {
        self.events.subscribe()
    }

    /// Borrow the underlying broadcast sender. Useful for tests that
    /// want to assert subscriber count; production code calls
    /// [`Self::subscribe`].
    pub fn event_sender(&self) -> &broadcast::Sender<FlowDefinitionEvent> {
        &self.events
    }

    /// Publish a draft body for `flow_id`. Returns the [`FlowRevisionId`]
    /// that is now (or already was) the flow's head, along with the
    /// classifier tag for tracing / event emission.
    ///
    /// Full contract (HR1 + HR2):
    ///
    /// 1. Parse the body into the typed [`FlowBody`] shape.
    /// 2. Resolve — every node's kind must be registered, every
    ///    node's settings must pass `validate_settings`, every link
    ///    endpoint must reference a declared node. Produces an
    ///    `Arc<FlowTopology>` ready to mount.
    /// 3. Canonicalise (RFC 8785 JCS) + blake3 hash.
    /// 4. Look up the current head; if its body hashes to the same
    ///    value, short-circuit (no `FlowStore` write, no swap, just
    ///    [`FlowDefinitionEvent::PublishShortCircuited`]).
    /// 5. Allocate a fresh [`FlowRevisionId`] and write through
    ///    [`FlowStore::put`].
    /// 6. Classify the edit relative to the previous head
    ///    (Initial / SettingsOnly / Structural / Mixed / Unchanged).
    /// 7. Apply per the classifier:
    ///    - Initial / Structural / Mixed — install the
    ///      freshly-resolved topology into [`ActiveTopologies`]
    ///      (atomic `ArcSwap` swap if previously mounted) and emit
    ///      [`FlowDefinitionEvent::SwapApplied`] carrying the
    ///      [`ApplyPolicy`] read from the *previous* body (HR4:
    ///      the body being torn down dictates how).
    ///    - SettingsOnly / Mixed — project the per-field deltas
    ///      onto the attached [`GraphStore`] via
    ///      [`WriteSlotOpts::config`]. HR3 order: structural swap
    ///      first, then writes.
    /// 8. Emit [`FlowDefinitionEvent::RevisionPublished`] tagged
    ///    with the classifier output.
    pub async fn publish(
        &self,
        flow_id: FlowId,
        body: serde_json::Value,
        source: DefinitionSource,
    ) -> Result<PublishOutcome, PublishError> {
        // `flow.definition.publish` per the Observability section.
        // `outcome` is recorded by `record_outcome` once the path
        // resolves; `kind` is recorded once classified.
        let span = info_span!(
            "flow.definition.publish",
            flow = %flow_id,
            revision = tracing::field::Empty,
            prev_head = tracing::field::Empty,
            source = %source.audit_tag(),
            kind = tracing::field::Empty,
            outcome = tracing::field::Empty,
        );
        self.publish_inner(flow_id, body, source).instrument(span).await
    }

    async fn publish_inner(
        &self,
        flow_id: FlowId,
        body: serde_json::Value,
        source: DefinitionSource,
    ) -> Result<PublishOutcome, PublishError> {
        let span = tracing::Span::current();
        // Step 1: parse the typed body.
        let parsed: FlowBody = match body::parse_body(&body) {
            Ok(b) => b,
            Err(e) => {
                let err = TopologyResolverError::BodyShape {
                    detail: e.to_string(),
                };
                span.record("outcome", "rejected");
                self.metrics.add_rejected();
                self.emit_rejected(&flow_id, &source, &err);
                return Err(err.into());
            }
        };

        // Step 2: resolve — keep the topology for the HR2 mount.
        let topology = match TopologyResolver::resolve_body(&parsed, &flow_id, &self.kinds).await {
            Ok(t) => t,
            Err(e) => {
                span.record("outcome", "rejected");
                self.metrics.add_rejected();
                self.metrics.add_resolve_failure();
                self.emit_rejected(&flow_id, &source, &e);
                return Err(e.into());
            }
        };

        // Step 3: canonicalise + hash.
        let draft_hash = body_hash(&body);

        // Step 4: look up the head + load it once for both the
        // short-circuit and the diff classifier.
        let prev_head = self.store.head(flow_id.clone()).await?;
        let prev_revision = match prev_head.as_ref() {
            Some(head) => Some(self.store.load(flow_id.clone(), Some(*head)).await?),
            None => None,
        };

        if let Some(prev) = prev_revision.as_ref() {
            let head_hash = body_hash(&prev.body);
            if head_hash == draft_hash {
                span.record("outcome", "short_circuited");
                span.record("revision", tracing::field::display(&prev.revision_id));
                self.metrics.add_short_circuited();
                debug!(
                    target: "starter_flow::definition",
                    head = %prev.revision_id,
                    body_hash = %draft_hash,
                    "publish short-circuited: draft body hash matches head"
                );
                let _ = self.events.send(FlowDefinitionEvent::PublishShortCircuited {
                    flow: flow_id.clone(),
                    head: prev.revision_id,
                    source,
                });
                return Ok(PublishOutcome::ShortCircuited {
                    head: prev.revision_id,
                });
            }
        }

        // Step 6 (preview): classify against the previous body.
        let edit = match prev_revision.as_ref() {
            None => EditKind::Initial,
            Some(prev) => match body::parse_body(&prev.body) {
                Ok(prev_parsed) => classify(&prev_parsed, &parsed),
                Err(e) => {
                    warn!(
                        target: "starter_flow::definition",
                        head = %prev.revision_id,
                        error = %e,
                        "prev head body failed to re-parse; treating as Structural"
                    );
                    EditKind::Structural
                }
            },
        };
        let kind_tag = edit.tag();
        span.record("kind", tracing::field::debug(&kind_tag));

        // Step 5: write a fresh revision BEFORE any side-effects.
        let revision_id = FlowRevisionId::new();
        let revision = FlowRevision::new(flow_id.clone(), revision_id, body)
            .with_source(source.audit_tag());
        let written = self.store.put(revision).await?;
        span.record("revision", tracing::field::display(&written));
        if let Some(prev) = prev_head.as_ref() {
            span.record("prev_head", tracing::field::display(prev));
        }

        // Emit RevisionPublished FIRST so consumers observe the
        // logical order revision-committed → topology-mounted →
        // settings-projected.
        info!(
            target: "starter_flow::definition",
            body_hash = %draft_hash,
            "publish accepted: new flow revision written"
        );
        let _ = self.events.send(FlowDefinitionEvent::RevisionPublished {
            flow: flow_id.clone(),
            revision: written,
            prev_head,
            source: source.clone(),
            kind: kind_tag,
        });

        // Step 7: dispatch on the classifier.
        let apply_policy = prev_revision
            .as_ref()
            .and_then(|prev| body::parse_body(&prev.body).ok())
            .map(|prev_parsed| prev_parsed.apply_policy)
            .unwrap_or_default();

        let do_swap = matches!(
            edit,
            EditKind::Initial | EditKind::Structural | EditKind::Mixed { .. }
        );
        let settings_writes: Vec<_> = match &edit {
            EditKind::SettingsOnly { writes } | EditKind::Mixed { writes } => writes.clone(),
            _ => Vec::new(),
        };

        if do_swap {
            self.swap_topology(
                flow_id.clone(),
                topology,
                written,
                prev_head,
                apply_policy,
                &source,
                &edit,
            )
            .await;
            self.metrics.add_swap();
        }

        if !settings_writes.is_empty() {
            self.apply_settings(&flow_id, &settings_writes).await?;
        }

        // HR-6: a successful publish supersedes any prior
        // ResolveFailed state for this flow.
        self.failed.write().await.remove(&flow_id);

        self.metrics.add_published();
        span.record("outcome", "published");
        debug!(
            target: "starter_flow::definition",
            settings_writes = settings_writes.len(),
            "publish dispatch complete"
        );

        Ok(PublishOutcome::Published {
            revision: written,
            prev_head,
            kind: kind_tag,
        })
    }

    /// Unmount a flow whose source has been removed (e.g. a YAML
    /// file deleted under a host-dir watcher).
    ///
    /// Per `DOCS/flow/scope/hot-reload.md` HR7: removing a file
    /// deletes the flow via `publish_delete(flow_id)` — the
    /// fourth method on the chokepoint. Semantics:
    ///
    /// 1. If the flow has a head in the [`FlowStore`], read its
    ///    `apply_policy` and fire `RunCancel` for every
    ///    registered run on that revision when the policy is
    ///    `Restart` (or `LiveMigrate` — unmount is structural).
    ///    `Drain` lets in-flight runs finish on the snapshot
    ///    they already hold.
    /// 2. Remove the [`ActiveTopologies`] entry. New runs will
    ///    fail to resolve until the flow is re-published.
    /// 3. Emit [`FlowDefinitionEvent::Removed`] on the bus.
    ///
    /// The append-only [`FlowStore`] is intentionally NOT
    /// touched — revisions are immutable per the SCOPE
    /// "Decisions made" block. If the flow is later re-published
    /// (e.g. the YAML file is restored), the new revision lands
    /// alongside the old ones and `boot_resume` picks it up.
    pub async fn publish_delete(
        &self,
        flow_id: FlowId,
        source: DefinitionSource,
    ) -> Result<(), PublishError> {
        let span = info_span!(
            "flow.definition.publish_delete",
            flow = %flow_id,
            source = %source.audit_tag(),
            cancelled_runs = tracing::field::Empty,
        );
        let _enter = span.enter();

        let prev_head = self.store.head(flow_id.clone()).await?;
        let apply_policy = if let Some(head) = prev_head {
            match self.store.load(flow_id.clone(), Some(head)).await {
                Ok(rev) => body::parse_body(&rev.body)
                    .map(|b| b.apply_policy)
                    .unwrap_or_default(),
                Err(_) => ApplyPolicy::default(),
            }
        } else {
            ApplyPolicy::default()
        };

        let cancelled = match (apply_policy, prev_head) {
            (ApplyPolicy::Restart | ApplyPolicy::LiveMigrate, Some(prev)) => {
                self.runs.cancel_for(&flow_id, &prev)
            }
            _ => 0,
        };
        span.record("cancelled_runs", cancelled);

        let removed = self.active.remove(&flow_id).await.is_some();
        // HR-6: deleting a flow also drops any ResolveFailed
        // bookkeeping for it; a subsequent re-register of the
        // missing kind must NOT remount a flow that was deleted
        // in the meantime.
        self.failed.write().await.remove(&flow_id);
        if removed {
            info!(
                target: "starter_flow::definition",
                cancelled_runs = cancelled,
                policy = ?apply_policy,
                "flow removed via publish_delete"
            );
        } else {
            debug!(
                target: "starter_flow::definition",
                "publish_delete: flow was not mounted; emitting Removed anyway"
            );
        }

        let _ = self.events.send(FlowDefinitionEvent::Removed {
            flow: flow_id,
            source,
        });
        Ok(())
    }

    /// Install a freshly-resolved topology and emit the swap event.
    /// Initial mounts also emit a `Mounted` event so consumers can
    /// distinguish first-time mount from subsequent swaps.
    ///
    /// The arity matches the publish-pipeline's bound context (flow,
    /// resolved topology, new + prev revision, apply policy, source,
    /// edit kind) — every parameter is consumed and grouping them
    /// into a struct just to satisfy clippy::too_many_arguments
    /// would obscure the call site without removing information.
    #[allow(clippy::too_many_arguments)]
    async fn swap_topology(
        &self,
        flow_id: FlowId,
        topology: Arc<crate::propagator::FlowTopology>,
        new_revision: FlowRevisionId,
        prev_head: Option<FlowRevisionId>,
        apply_policy: ApplyPolicy,
        source: &DefinitionSource,
        edit: &EditKind,
    ) {
        let span = info_span!(
            "flow.definition.swap",
            flow = %flow_id,
            from_revision = ?prev_head.as_ref().map(ToString::to_string),
            to_revision = %new_revision,
            apply_policy = ?apply_policy,
        );
        let _enter = span.enter();

        self.active.install(flow_id.clone(), topology).await;

        match prev_head {
            None => {
                info!(
                    target: "starter_flow::definition",
                    source = %source.audit_tag(),
                    "flow mounted (initial publish)"
                );
                let _ = self.events.send(FlowDefinitionEvent::Mounted {
                    flow: flow_id.clone(),
                    revision: new_revision,
                });
            }
            Some(_) => {
                info!(
                    target: "starter_flow::definition",
                    edit_kind = ?edit.tag(),
                    "active topology swapped"
                );
            }
        }

        // HR-4 apply_policy dispatch. The policy is read from the
        // *previous* revision; the rules for what to do are:
        //
        // - `Drain`   — in-flight runs finish on their snapshot;
        //               nothing extra to do here.
        // - `Restart` — fire `RunCancel` for every run still
        //               registered against `prev_head`.
        // - `LiveMigrate` — falls back to `Restart` for the
        //               structural piece of the swap. The settings
        //               piece (`SettingsOnly`) takes the
        //               apply_settings path below and does not
        //               cancel in-flight runs.
        let cancelled = match (apply_policy, prev_head) {
            (ApplyPolicy::Restart, Some(prev)) => self.runs.cancel_for(&flow_id, &prev),
            (ApplyPolicy::LiveMigrate, Some(prev))
                if matches!(edit, EditKind::Structural | EditKind::Mixed { .. }) =>
            {
                self.runs.cancel_for(&flow_id, &prev)
            }
            _ => 0,
        };
        if cancelled > 0 {
            info!(
                target: "starter_flow::definition",
                cancelled_runs = cancelled,
                policy = ?apply_policy,
                "swap cancelled in-flight runs per apply_policy"
            );
        }

        let _ = self.events.send(FlowDefinitionEvent::SwapApplied {
            flow: flow_id,
            from_revision: prev_head,
            to_revision: new_revision,
            apply_policy,
        });
    }

    /// Walk [`FlowStore::list`] and mount every flow's head per
    /// `DOCS/flow/scope/hot-reload.md` HR5 (*"boot is resume to
    /// last known good"*).
    ///
    /// For each known [`FlowId`]:
    ///
    /// 1. Load the head [`FlowRevision`] (skip if the flow has
    ///    no head — it has never been published).
    /// 2. Resolve it via [`TopologyResolver::resolve`].
    ///    On success, install into [`ActiveTopologies`] and emit
    ///    [`FlowDefinitionEvent::Mounted`].
    ///    On failure, emit
    ///    [`FlowDefinitionEvent::ResolveFailed`] and continue —
    ///    one bad flow does not abort the walk (HR6's *"one
    ///    bad revision never poisons the flow"* lifted to the
    ///    boot-resume scope).
    ///
    /// Returns the [`BootResumeReport`] so the engine can log a
    /// single startup line and tests can assert outcomes.
    ///
    /// Failure to even *list* flows from the [`FlowStore`]
    /// (backend unavailable) is surfaced as the typed error;
    /// hosts that want the engine to come up degraded anyway
    /// can handle it and continue.
    pub async fn boot_resume(&self) -> Result<BootResumeReport, FlowError> {
        let flows = self.store.list().await?;
        let mut report = BootResumeReport::default();
        for flow_id in flows {
            let head = match self.store.head(flow_id.clone()).await? {
                Some(h) => h,
                None => {
                    // Flow exists in the store but has no head
                    // — nothing to mount.
                    report.skipped += 1;
                    continue;
                }
            };
            let revision = match self.store.load(flow_id.clone(), Some(head)).await {
                Ok(r) => r,
                Err(e) => {
                    warn!(
                        target: "starter_flow::definition",
                        flow = %flow_id,
                        revision = %head,
                        error = %e,
                        "boot_resume: failed to load head revision"
                    );
                    let _ = self.events.send(FlowDefinitionEvent::ResolveFailed {
                        flow: flow_id.clone(),
                        revision: head,
                        error: e.to_string(),
                    });
                    self.failed.write().await.insert(flow_id, head);
                    self.metrics.add_resolve_failure();
                    report.failed += 1;
                    continue;
                }
            };
            match TopologyResolver::resolve(&revision, &self.kinds).await {
                Ok(topology) => {
                    self.active.install(flow_id.clone(), topology).await;
                    self.failed.write().await.remove(&flow_id);
                    info!(
                        target: "starter_flow::definition",
                        flow = %flow_id,
                        revision = %head,
                        source = %revision.source,
                        "boot_resume: mounted flow head"
                    );
                    let _ = self.events.send(FlowDefinitionEvent::Mounted {
                        flow: flow_id,
                        revision: head,
                    });
                    report.mounted += 1;
                }
                Err(e) => {
                    warn!(
                        target: "starter_flow::definition",
                        flow = %flow_id,
                        revision = %head,
                        error = %e,
                        "boot_resume: head revision failed to resolve"
                    );
                    let _ = self.events.send(FlowDefinitionEvent::ResolveFailed {
                        flow: flow_id.clone(),
                        revision: head,
                        error: e.to_string(),
                    });
                    self.failed.write().await.insert(flow_id, head);
                    self.metrics.add_resolve_failure();
                    report.failed += 1;
                }
            }
        }
        info!(
            target: "starter_flow::definition",
            mounted = report.mounted,
            failed = report.failed,
            skipped = report.skipped,
            "boot_resume complete"
        );
        Ok(report)
    }

    /// HR-6 / HR8 first paragraph: a node kind was registered.
    /// Walk every flow currently in `ResolveFailed` state and
    /// re-attempt [`TopologyResolver::resolve`] against the live
    /// `NodeKindRegistry`. Successful resolves install into
    /// [`ActiveTopologies`] and emit
    /// [`FlowDefinitionEvent::Mounted`].
    ///
    /// Returns the number of flows that successfully remounted.
    /// Hosts call this from the wrapper around
    /// [`crate::registry::NodeKindRegistry::register`] (the
    /// registry itself doesn't depend on the definition layer).
    pub async fn on_kind_registered(&self, _kind: &starter_flow_spi::node::KindId) -> usize {
        // Snapshot the failed map so we can release the lock
        // before doing async work.
        let candidates: Vec<(FlowId, FlowRevisionId)> = {
            let guard = self.failed.read().await;
            guard.iter().map(|(f, r)| (f.clone(), *r)).collect()
        };
        let mut remounted = 0usize;
        for (flow_id, revision_id) in candidates {
            let revision = match self.store.load(flow_id.clone(), Some(revision_id)).await {
                Ok(r) => r,
                Err(_) => continue,
            };
            match TopologyResolver::resolve(&revision, &self.kinds).await {
                Ok(topology) => {
                    self.active.install(flow_id.clone(), topology).await;
                    self.failed.write().await.remove(&flow_id);
                    info!(
                        target: "starter_flow::definition",
                        flow = %flow_id,
                        revision = %revision_id,
                        "kind registration remounted previously-failed flow"
                    );
                    let _ = self.events.send(FlowDefinitionEvent::Mounted {
                        flow: flow_id,
                        revision: revision_id,
                    });
                    remounted += 1;
                }
                Err(_) => {
                    // Still can't resolve — leave it in the
                    // failed map for the next kind registration.
                }
            }
        }
        if remounted > 0 {
            info!(
                target: "starter_flow::definition",
                remounted,
                "on_kind_registered: walk complete"
            );
        }
        remounted
    }

    /// HR-6 / HR8 second paragraph: a node kind was deregistered.
    /// Walk every [`ActiveTopologies`] entry; for each topology
    /// that references the kind:
    ///
    /// 1. Apply the flow's `apply_policy` per HR4 (
    ///    `drain` lets in-flight runs finish on the snapshot they
    ///    already hold — which is safe because behaviors live
    ///    inside the topology snapshot, not in the registry;
    ///    `restart` cancels via [`RunRegistry::cancel_for`];
    ///    `live-migrate` falls back to `restart` for deregister
    ///    because it is structural by definition).
    /// 2. Remove the [`ActiveTopologies`] entry; add the flow's
    ///    head to the `failed` map so a subsequent re-register
    ///    of the kind can remount it.
    /// 3. Emit [`FlowDefinitionEvent::KindRevoked`] and
    ///    [`FlowDefinitionEvent::ResolveFailed`].
    ///
    /// Returns the number of flows revoked.
    pub async fn on_kind_deregistered(
        &self,
        kind: &starter_flow_spi::node::KindId,
    ) -> usize {
        // Snapshot the flow ids first; releasing the lock keeps
        // the walk lock-free during the async store load below.
        let active_ids = self.active.snapshot_ids().await;
        let mut revoked = 0usize;
        for flow_id in active_ids {
            let active = match self.active.get(&flow_id).await {
                Some(a) => a,
                None => continue,
            };
            let topology = active.load();
            let references_kind = topology
                .behaviors
                .values()
                .any(|behavior| behavior.kind_id() == kind);
            if !references_kind {
                continue;
            }

            // Read the flow's head + apply_policy.
            let head = match self.store.head(flow_id.clone()).await {
                Ok(Some(h)) => h,
                _ => continue,
            };
            let apply_policy = match self.store.load(flow_id.clone(), Some(head)).await {
                Ok(rev) => body::parse_body(&rev.body)
                    .map(|b| b.apply_policy)
                    .unwrap_or_default(),
                Err(_) => ApplyPolicy::default(),
            };

            let cancelled = match apply_policy {
                ApplyPolicy::Restart | ApplyPolicy::LiveMigrate => {
                    self.runs.cancel_for(&flow_id, &head)
                }
                ApplyPolicy::Drain => 0,
                _ => 0,
            };

            self.active.remove(&flow_id).await;
            self.failed.write().await.insert(flow_id.clone(), head);

            let span = info_span!(
                "flow.definition.kind_revoked",
                flow = %flow_id,
                kind = %kind,
                apply_policy = ?apply_policy,
                cancelled_runs = cancelled,
            );
            let _enter = span.enter();
            info!(
                target: "starter_flow::definition",
                "kind deregistration revoked active topology"
            );

            let _ = self.events.send(FlowDefinitionEvent::KindRevoked {
                flow: flow_id.clone(),
                kind: kind.to_string(),
                apply_policy,
            });
            let _ = self.events.send(FlowDefinitionEvent::ResolveFailed {
                flow: flow_id,
                revision: head,
                error: format!("kind `{kind}` was deregistered"),
            });
            self.metrics.add_resolve_failure();
            revoked += 1;
        }
        revoked
    }

    /// Snapshot the set of flow ids currently tracked as
    /// `ResolveFailed`. Exposed for tests and host introspection
    /// (e.g. an admin endpoint that surfaces unmounted flows).
    pub async fn failed_flows(&self) -> Vec<(FlowId, FlowRevisionId)> {
        let guard = self.failed.read().await;
        guard.iter().map(|(f, r)| (f.clone(), *r)).collect()
    }

    /// Project a settings delta onto the live [`GraphStore`].
    async fn apply_settings(
        &self,
        flow_id: &FlowId,
        writes: &[(starter_flow_spi::node::SlotRef, starter_flow_spi::node::SlotValue)],
    ) -> Result<(), PublishError> {
        let Some(graph) = self.graph.as_ref() else {
            debug!(
                target: "starter_flow::definition",
                flow = %flow_id,
                writes = writes.len(),
                "skipping settings projection: no GraphStore attached"
            );
            return Ok(());
        };
        for (slot, value) in writes {
            graph
                .write_slot(slot, value.clone(), WriteSlotOpts::config())
                .await
                .map_err(PublishError::Graph)?;
        }
        Ok(())
    }

    /// Compute the canonical-body blake3 hash for a value without
    /// publishing. Exposed for tests + future HR-2 callers that
    /// want to compare hashes outside the publish flow.
    pub fn hash_body(body: &serde_json::Value) -> BodyHash {
        body_hash(body)
    }

    fn emit_rejected(
        &self,
        flow_id: &FlowId,
        source: &DefinitionSource,
        err: &TopologyResolverError,
    ) {
        let span = info_span!(
            "flow.definition.resolve_failed",
            flow = %flow_id,
            source = %source.audit_tag(),
            error = %err,
        );
        let _enter = span.enter();
        warn!(
            target: "starter_flow::definition",
            "publish rejected"
        );
        let _ = self.events.send(FlowDefinitionEvent::Rejected {
            flow: flow_id.clone(),
            source: source.clone(),
            reason: err.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;

    use starter_flow_spi::Cancel;

    use starter_flow_spi::flow::FlowResult;
    use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap};

    /// Minimal in-memory `FlowStore` for tests. Mirrors the shape
    /// the SQLite impl will land in Phase HR-3; sufficient for HR-1
    /// smoke coverage.
    #[derive(Default)]
    struct MemStore {
        // (flow_id → revisions in insertion order; last = head).
        inner: Mutex<HashMap<FlowId, Vec<FlowRevision>>>,
    }

    impl MemStore {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn revision_count(&self, flow: &FlowId) -> usize {
            self.inner
                .lock()
                .unwrap()
                .get(flow)
                .map(Vec::len)
                .unwrap_or(0)
        }
    }

    #[async_trait]
    impl FlowStore for MemStore {
        async fn load(
            &self,
            flow_id: FlowId,
            revision: Option<FlowRevisionId>,
        ) -> FlowResult<FlowRevision> {
            let guard = self.inner.lock().unwrap();
            let revs = guard.get(&flow_id).ok_or_else(|| FlowError::NotFound {
                kind: "flow",
                id: flow_id.to_string(),
            })?;
            let target = match revision {
                Some(r) => revs.iter().find(|x| x.revision_id == r).cloned(),
                None => revs.last().cloned(),
            };
            target.ok_or_else(|| FlowError::NotFound {
                kind: "revision",
                id: flow_id.to_string(),
            })
        }

        async fn put(&self, revision: FlowRevision) -> FlowResult<FlowRevisionId> {
            let mut guard = self.inner.lock().unwrap();
            let revs = guard.entry(revision.flow_id.clone()).or_default();
            let id = revision.revision_id;
            revs.push(revision);
            Ok(id)
        }

        async fn list(&self) -> FlowResult<Vec<FlowId>> {
            Ok(self.inner.lock().unwrap().keys().cloned().collect())
        }

        async fn revisions(&self, flow_id: FlowId) -> FlowResult<Vec<FlowRevisionId>> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(&flow_id)
                .map(|v| v.iter().rev().map(|r| r.revision_id).collect())
                .unwrap_or_default())
        }

        async fn head(&self, flow_id: FlowId) -> FlowResult<Option<FlowRevisionId>> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(&flow_id)
                .and_then(|v| v.last().map(|r| r.revision_id)))
        }
    }

    struct AnyKind {
        kind: KindId,
    }
    impl AnyKind {
        fn arc(s: &str) -> Arc<Self> {
            Arc::new(Self {
                kind: KindId::new(s).unwrap(),
            })
        }
    }
    #[async_trait]
    impl NodeBehavior for AnyKind {
        fn kind_id(&self) -> &KindId {
            &self.kind
        }
        async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
            Ok(SlotMap::new())
        }
    }

    fn flow_id() -> FlowId {
        FlowId::new("examples.test.demo").unwrap()
    }

    fn body_v1() -> serde_json::Value {
        serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any"}
            ],
            "links": [{"from": "test.n1.out", "to": "test.n2.in"}]
        })
    }

    fn body_v1_reordered_keys() -> serde_json::Value {
        // Same body, different key order at every level.
        serde_json::json!({
            "links": [{"to": "test.n2.in", "from": "test.n1.out"}],
            "nodes": [
                {"kind": "com.example.any", "id": "test.n1"},
                {"kind": "com.example.any", "id": "test.n2"}
            ],
            "flow_id": "examples.test.demo"
        })
    }

    async fn build_manager() -> (Arc<DefinitionManager>, Arc<MemStore>) {
        let store = MemStore::new();
        let kinds = Arc::new(NodeKindRegistry::new());
        kinds.register(AnyKind::arc("com.example.any")).await.unwrap();
        let mgr = Arc::new(DefinitionManager::new(store.clone(), kinds));
        (mgr, store)
    }

    /// HR1: idempotent publish is a no-op.
    #[tokio::test]
    async fn hr1_idempotent_publish_is_noop() {
        let (mgr, store) = build_manager().await;

        let mut rx = mgr.subscribe();

        let first = mgr
            .publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .expect("first publish");
        let first_id = match first {
            PublishOutcome::Published { revision, .. } => revision,
            other => panic!("expected Published, got {other:?}"),
        };
        assert_eq!(store.revision_count(&flow_id()), 1);

        // Same body, same source — must short-circuit.
        let second = mgr
            .publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .expect("second publish");
        assert_eq!(
            second,
            PublishOutcome::ShortCircuited { head: first_id },
            "duplicate publish must short-circuit"
        );
        assert_eq!(
            store.revision_count(&flow_id()),
            1,
            "short-circuit must not write a second revision"
        );

        // Bus shape: first publish emits RevisionPublished +
        // Mounted + SwapApplied (in that order); second publish
        // emits PublishShortCircuited. Never RevisionPublished
        // twice.
        let ev1 = rx.recv().await.expect("event 1");
        assert!(matches!(ev1, FlowDefinitionEvent::RevisionPublished { .. }));
        let ev2 = rx.recv().await.expect("event 2");
        assert!(matches!(ev2, FlowDefinitionEvent::Mounted { .. }));
        let ev3 = rx.recv().await.expect("event 3");
        assert!(matches!(ev3, FlowDefinitionEvent::SwapApplied { .. }));
        let ev4 = rx.recv().await.expect("event 4");
        assert!(matches!(
            ev4,
            FlowDefinitionEvent::PublishShortCircuited { .. }
        ));
    }

    /// HR1: bad revision never goes live.
    #[tokio::test]
    async fn hr1_bad_revision_never_goes_live() {
        let (mgr, store) = build_manager().await;
        // First publish lands a clean head so we can verify it
        // doesn't move on the bad publish.
        let first = mgr
            .publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();
        let first_id = match first {
            PublishOutcome::Published { revision, .. } => revision,
            other => panic!("expected Published, got {other:?}"),
        };

        // Publish a body referencing an unregistered kind.
        let bad = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [{"id": "test.n1", "kind": "com.missing"}],
            "links": []
        });
        let err = mgr
            .publish(flow_id(), bad, DefinitionSource::Api)
            .await
            .expect_err("bad publish must error");
        assert!(matches!(
            err,
            PublishError::Resolve(TopologyResolverError::UnknownKind { .. })
        ));

        // Head is unchanged; no second row in the store.
        assert_eq!(store.revision_count(&flow_id()), 1);
        assert_eq!(store.head(flow_id()).await.unwrap(), Some(first_id));
    }

    /// HR1: canonicalisation collapses semantically-equal bodies in
    /// different key orders onto the same revision.
    #[tokio::test]
    async fn hr1_canonical_publish_dedupes_key_order() {
        let (mgr, store) = build_manager().await;

        mgr.publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();
        let second = mgr
            .publish(flow_id(), body_v1_reordered_keys(), DefinitionSource::Api)
            .await
            .expect("re-ordered keys publish");
        assert!(matches!(second, PublishOutcome::ShortCircuited { .. }));
        assert_eq!(store.revision_count(&flow_id()), 1);
    }

    /// HR1: a structural delta (new node) writes a fresh revision.
    #[tokio::test]
    async fn hr1_structural_change_writes_new_revision() {
        let (mgr, store) = build_manager().await;
        mgr.publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();

        let body_v2 = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any"},
                {"id": "test.n3", "kind": "com.example.any"}
            ],
            "links": [
                {"from": "test.n1.out", "to": "test.n2.in"},
                {"from": "test.n2.out", "to": "test.n3.in"}
            ]
        });
        let r = mgr
            .publish(flow_id(), body_v2, DefinitionSource::Api)
            .await
            .expect("structural publish");
        match r {
            PublishOutcome::Published { kind, prev_head, .. } => {
                assert_eq!(kind, EditKindTag::Structural);
                assert!(prev_head.is_some());
            }
            other => panic!("expected Published, got {other:?}"),
        }
        assert_eq!(store.revision_count(&flow_id()), 2);
    }

    /// HR1: flow_id in body must match the publish target.
    #[tokio::test]
    async fn hr1_flow_id_mismatch_rejected() {
        let (mgr, _store) = build_manager().await;
        let bad = serde_json::json!({
            "flow_id": "examples.test.other",
            "nodes": [], "links": []
        });
        let err = mgr
            .publish(flow_id(), bad, DefinitionSource::Api)
            .await
            .expect_err("flow id mismatch");
        assert!(matches!(
            err,
            PublishError::Resolve(TopologyResolverError::FlowIdMismatch { .. })
        ));
    }

    // ===================================================================
    // HR-2 smoke tests
    // ===================================================================

    use crate::graph::InMemoryGraphStore;
    use starter_flow_spi::graph::SubscribeOpts;
    use starter_flow_spi::node::SlotValue;
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::timeout;

    async fn build_manager_with_graph() -> (
        Arc<DefinitionManager>,
        Arc<MemStore>,
        Arc<InMemoryGraphStore>,
    ) {
        let store = MemStore::new();
        let kinds = Arc::new(NodeKindRegistry::new());
        kinds
            .register(AnyKind::arc("com.example.any"))
            .await
            .unwrap();
        let graph: Arc<InMemoryGraphStore> = Arc::new(InMemoryGraphStore::new());
        let mgr = Arc::new(DefinitionManager::with_graph(
            store.clone(),
            kinds,
            graph.clone(),
        ));
        (mgr, store, graph)
    }

    fn body_with_setting(value: &str) -> serde_json::Value {
        serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any",
                 "settings": {"prompt": value}},
                {"id": "test.n2", "kind": "com.example.any"}
            ],
            "links": [{"from": "test.n1.out", "to": "test.n2.in"}]
        })
    }

    /// HR2: a settings-only edit fires exactly one `SlotChanged`
    /// event per delta and does NOT swap the active topology.
    #[tokio::test]
    async fn hr2_settings_edit_is_one_slot_write() {
        let (mgr, store, graph) = build_manager_with_graph().await;

        // Mount.
        mgr.publish(flow_id(), body_with_setting("old"), DefinitionSource::Api)
            .await
            .unwrap();
        let active_after_mount = mgr
            .active_topologies()
            .get(&flow_id())
            .await
            .expect("flow mounted")
            .load();

        // Subscribe to the graph AFTER the mount so we don't see
        // any initial-projection writes (HR-2's resolver doesn't
        // do an initial projection yet — that's HR-7 follow-up —
        // but the subscription point keeps the test honest).
        let mut graph_rx = graph.subscribe(SubscribeOpts::default());

        // Subscribe to the definition bus.
        let mut def_rx = mgr.subscribe();

        // Settings-only edit.
        let out = mgr
            .publish(flow_id(), body_with_setting("new"), DefinitionSource::Api)
            .await
            .expect("settings publish");
        assert!(matches!(
            out,
            PublishOutcome::Published {
                kind: EditKindTag::Settings,
                ..
            }
        ));
        // A new revision did land (the canonical hash differs).
        assert_eq!(store.revision_count(&flow_id()), 2);

        // The active topology pointer must NOT have been swapped.
        let active_after_edit = mgr
            .active_topologies()
            .get(&flow_id())
            .await
            .expect("still mounted")
            .load();
        assert!(
            Arc::ptr_eq(&active_after_mount, &active_after_edit),
            "settings-only edit must not swap the active topology"
        );

        // Exactly one SlotChanged event for the prompt slot.
        let ev = timeout(Duration::from_millis(200), graph_rx.next())
            .await
            .expect("graph event")
            .expect("envelope");
        let value = ev.value.expect("event carries a value");
        assert_eq!(ev.slot.node.as_str(), "test.n1");
        assert_eq!(ev.slot.slot, "prompt");
        assert!(matches!(value, SlotValue::String(s) if s == "new"));

        // RevisionPublished tagged Settings; NO SwapApplied (the
        // mount's own SwapApplied was already consumed before
        // def_rx subscribed).
        let def_ev = def_rx.recv().await.expect("def event");
        assert!(matches!(
            def_ev,
            FlowDefinitionEvent::RevisionPublished {
                kind: EditKindTag::Settings,
                ..
            }
        ));
        // No further definition events for this publish.
        assert!(timeout(Duration::from_millis(50), def_rx.recv()).await.is_err());
    }

    /// HR2: a structural edit swaps the active topology in place
    /// and emits SwapApplied (no settings writes).
    #[tokio::test]
    async fn hr2_structural_edit_swaps_active_topology() {
        let (mgr, _store, graph) = build_manager_with_graph().await;

        mgr.publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();
        let active = mgr.active_topologies().get(&flow_id()).await.unwrap();
        let before = active.load();

        let mut graph_rx = graph.subscribe(SubscribeOpts::default());
        let mut def_rx = mgr.subscribe();

        let v2 = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any"},
                {"id": "test.n3", "kind": "com.example.any"}
            ],
            "links": [
                {"from": "test.n1.out", "to": "test.n2.in"},
                {"from": "test.n2.out", "to": "test.n3.in"}
            ]
        });
        let out = mgr
            .publish(flow_id(), v2, DefinitionSource::Api)
            .await
            .unwrap();
        assert!(matches!(
            out,
            PublishOutcome::Published {
                kind: EditKindTag::Structural,
                ..
            }
        ));

        let after = active.load();
        assert!(
            !Arc::ptr_eq(&before, &after),
            "structural edit must swap the topology pointer"
        );

        // No slot writes.
        assert!(timeout(Duration::from_millis(50), graph_rx.next()).await.is_err());

        // RevisionPublished + SwapApplied (in that order).
        let mut saw_published = false;
        let mut saw_swap = false;
        for _ in 0..2 {
            let ev = def_rx.recv().await.expect("def event");
            match ev {
                FlowDefinitionEvent::RevisionPublished { kind, .. } => {
                    assert_eq!(kind, EditKindTag::Structural);
                    saw_published = true;
                }
                FlowDefinitionEvent::SwapApplied {
                    from_revision,
                    apply_policy,
                    ..
                } => {
                    assert!(from_revision.is_some(), "second mount must have a prev");
                    assert_eq!(apply_policy, ApplyPolicy::Drain);
                    saw_swap = true;
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
        assert!(saw_published && saw_swap);
    }

    /// HR2: a mixed edit (new node + settings change on a
    /// wiring-stable node) fires SwapApplied AND the settings
    /// write.
    #[tokio::test]
    async fn hr2_mixed_edit_swaps_then_writes_settings() {
        let (mgr, _store, graph) = build_manager_with_graph().await;

        // Body with two nodes; n2 has a settings field we'll
        // edit. (n2's wiring will stay stable across the edit; n1
        // gains a new outbound link.)
        let v1 = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any",
                 "settings": {"level": "info"}}
            ],
            "links": [{"from": "test.n1.out", "to": "test.n2.in"}]
        });
        mgr.publish(flow_id(), v1, DefinitionSource::Api).await.unwrap();

        let mut graph_rx = graph.subscribe(SubscribeOpts::default());

        // v2: add n3 + a new link off n1 (n1 wiring shifts), AND
        // change n2.level (n2 wiring stable).
        let v2 = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any",
                 "settings": {"level": "debug"}},
                {"id": "test.n3", "kind": "com.example.any"}
            ],
            "links": [
                {"from": "test.n1.out", "to": "test.n2.in"},
                {"from": "test.n1.out", "to": "test.n3.in"}
            ]
        });
        let out = mgr.publish(flow_id(), v2, DefinitionSource::Api).await.unwrap();
        assert!(matches!(
            out,
            PublishOutcome::Published {
                kind: EditKindTag::Mixed,
                ..
            }
        ));

        // Settings projection fires (only for wiring-stable n2).
        let ev = timeout(Duration::from_millis(200), graph_rx.next())
            .await
            .expect("graph event")
            .expect("envelope");
        let value = ev.value.expect("event carries a value");
        assert_eq!(ev.slot.node.as_str(), "test.n2");
        assert_eq!(ev.slot.slot, "level");
        assert!(matches!(value, SlotValue::String(s) if s == "debug"));
    }

    /// HR2: initial publish emits Mounted + SwapApplied with no
    /// previous revision.
    #[tokio::test]
    async fn hr2_initial_publish_emits_mounted() {
        let (mgr, _store, _graph) = build_manager_with_graph().await;
        let mut def_rx = mgr.subscribe();

        mgr.publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();

        let mut saw_mounted = false;
        let mut saw_swap = false;
        let mut saw_published = false;
        for _ in 0..3 {
            let ev = def_rx.recv().await.expect("event");
            match ev {
                FlowDefinitionEvent::Mounted { .. } => saw_mounted = true,
                FlowDefinitionEvent::SwapApplied { from_revision, .. } => {
                    assert!(from_revision.is_none());
                    saw_swap = true;
                }
                FlowDefinitionEvent::RevisionPublished { kind, .. } => {
                    assert_eq!(kind, EditKindTag::Initial);
                    saw_published = true;
                }
                other => panic!("unexpected: {other:?}"),
            }
        }
        assert!(saw_mounted && saw_swap && saw_published);
    }

    // ===================================================================
    // HR-4 smoke tests — apply_policy dispatch + boot resume
    // ===================================================================

    fn body_with_policy(value: &str, policy: &str) -> serde_json::Value {
        serde_json::json!({
            "flow_id": "examples.test.demo",
            "apply_policy": policy,
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any",
                 "settings": {"prompt": value}},
                {"id": "test.n2", "kind": "com.example.any"}
            ],
            "links": [{"from": "test.n1.out", "to": "test.n2.in"}]
        })
    }

    fn structural_body_with_policy(policy: &str) -> serde_json::Value {
        serde_json::json!({
            "flow_id": "examples.test.demo",
            "apply_policy": policy,
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any"},
                {"id": "test.n3", "kind": "com.example.any"}
            ],
            "links": [
                {"from": "test.n1.out", "to": "test.n2.in"},
                {"from": "test.n2.out", "to": "test.n3.in"}
            ]
        })
    }

    /// HR-4: `apply_policy: drain` (default) leaves in-flight runs
    /// alone on a structural swap — registered `RunCancel` handles
    /// are NOT fired.
    #[tokio::test]
    async fn hr4_drain_policy_does_not_cancel_in_flight_runs() {
        let (mgr, _store, _graph) = build_manager_with_graph().await;
        let flow = flow_id();

        // First publish — apply_policy defaults to drain.
        let first = mgr
            .publish(flow.clone(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();
        let prev_rev = match first {
            PublishOutcome::Published { revision, .. } => revision,
            other => panic!("expected Published, got {other:?}"),
        };

        // Pretend an in-flight run is executing against prev_rev.
        let cancel = crate::run::RunCancel::new();
        let _reg = mgr.register_run(flow.clone(), prev_rev, cancel.clone());

        // Structural edit lands a swap. Default policy is drain.
        let _ = mgr
            .publish(flow.clone(), structural_body_with_policy("drain"), DefinitionSource::Api)
            .await
            .unwrap();

        assert!(
            !cancel.is_cancelled(),
            "drain policy must NOT cancel in-flight runs"
        );
    }

    /// HR-4: `apply_policy: restart` cancels every in-flight run
    /// against the previous revision on a structural swap, and the
    /// `SwapApplied` event still carries the (old) policy.
    #[tokio::test]
    async fn hr4_restart_policy_cancels_in_flight_runs() {
        let (mgr, _store, _graph) = build_manager_with_graph().await;
        let flow = flow_id();

        // First publish carries apply_policy: restart so the *next*
        // structural edit (which is the one that triggers dispatch)
        // is governed by `restart`.
        let first = mgr
            .publish(flow.clone(), body_with_policy("v1", "restart"), DefinitionSource::Api)
            .await
            .unwrap();
        let prev_rev = match first {
            PublishOutcome::Published { revision, .. } => revision,
            other => panic!("expected Published, got {other:?}"),
        };

        // Two in-flight runs against prev_rev.
        let c1 = crate::run::RunCancel::new();
        let c2 = crate::run::RunCancel::new();
        let _r1 = mgr.register_run(flow.clone(), prev_rev, c1.clone());
        let _r2 = mgr.register_run(flow.clone(), prev_rev, c2.clone());

        let mut def_rx = mgr.subscribe();

        // Structural edit (adds a node) — must trigger Restart
        // dispatch using the OLD revision's policy.
        let _ = mgr
            .publish(flow.clone(), structural_body_with_policy("restart"), DefinitionSource::Api)
            .await
            .unwrap();

        assert!(c1.is_cancelled(), "restart must fire RunCancel for in-flight run 1");
        assert!(c2.is_cancelled(), "restart must fire RunCancel for in-flight run 2");

        // SwapApplied event carries the policy that governed the swap.
        let mut saw_swap_with_restart = false;
        for _ in 0..3 {
            if let Ok(Ok(FlowDefinitionEvent::SwapApplied { apply_policy, .. })) =
                timeout(Duration::from_millis(100), def_rx.recv()).await
            {
                if matches!(apply_policy, ApplyPolicy::Restart) {
                    saw_swap_with_restart = true;
                }
            }
        }
        assert!(saw_swap_with_restart, "SwapApplied must carry the restart policy");
    }

    /// HR-4: `apply_policy: live-migrate` keeps in-flight runs
    /// alive when only settings change (wiring is stable), but
    /// falls back to `restart` for structural deltas.
    #[tokio::test]
    async fn hr4_live_migrate_falls_back_to_restart_for_structural() {
        let (mgr, _store, _graph) = build_manager_with_graph().await;
        let flow = flow_id();

        // Mount with live-migrate policy.
        let first = mgr
            .publish(flow.clone(), body_with_policy("v1", "live-migrate"), DefinitionSource::Api)
            .await
            .unwrap();
        let rev1 = match first {
            PublishOutcome::Published { revision, .. } => revision,
            other => panic!("expected Published, got {other:?}"),
        };

        // Settings-only edit: wiring stable; in-flight run must NOT
        // be cancelled.
        let c_settings = crate::run::RunCancel::new();
        let _rs = mgr.register_run(flow.clone(), rev1, c_settings.clone());
        let _ = mgr
            .publish(flow.clone(), body_with_policy("v2", "live-migrate"), DefinitionSource::Api)
            .await
            .unwrap();
        assert!(
            !c_settings.is_cancelled(),
            "live-migrate + settings-only must not cancel runs"
        );

        // Now do a structural edit: live-migrate must fall back to
        // restart and cancel the previous revision's runs.
        let head_after_settings = mgr.store().head(flow.clone()).await.unwrap().unwrap();
        let c_structural = crate::run::RunCancel::new();
        let _rstruct = mgr.register_run(flow.clone(), head_after_settings, c_structural.clone());
        let _ = mgr
            .publish(flow.clone(), structural_body_with_policy("live-migrate"), DefinitionSource::Api)
            .await
            .unwrap();
        assert!(
            c_structural.is_cancelled(),
            "live-migrate + structural must fall back to restart and cancel runs"
        );
    }

    /// HR-4 (HR5 in the doc): `boot_resume` mounts every flow's
    /// head and emits `Mounted` events. Flows whose heads fail to
    /// resolve emit `ResolveFailed` instead and don't abort the
    /// walk.
    #[tokio::test]
    async fn hr4_boot_resume_mounts_known_flows() {
        let (mgr, _store, _graph) = build_manager_with_graph().await;

        // Seed two flows via publish (then drop the active mounts
        // to simulate a fresh boot).
        let flow_a = FlowId::new("examples.boot.a").unwrap();
        let flow_b = FlowId::new("examples.boot.b").unwrap();
        let body_a = serde_json::json!({
            "flow_id": "examples.boot.a",
            "nodes": [{"id": "boot.n", "kind": "com.example.any"}],
            "links": []
        });
        let body_b = serde_json::json!({
            "flow_id": "examples.boot.b",
            "nodes": [{"id": "boot.n", "kind": "com.example.any"}],
            "links": []
        });
        mgr.publish(flow_a.clone(), body_a, DefinitionSource::Api)
            .await
            .unwrap();
        mgr.publish(flow_b.clone(), body_b, DefinitionSource::Api)
            .await
            .unwrap();

        // Wipe the active map so boot_resume has work to do (a
        // real boot would have an empty ActiveTopologies anyway —
        // this just isolates the test from the publish-time mount).
        let _ = mgr.active_topologies().remove(&flow_a).await;
        let _ = mgr.active_topologies().remove(&flow_b).await;
        assert_eq!(mgr.active_topologies().len().await, 0);

        let mut rx = mgr.subscribe();
        let report = mgr.boot_resume().await.expect("boot_resume");
        assert_eq!(report.mounted, 2);
        assert_eq!(report.failed, 0);
        assert_eq!(report.skipped, 0);
        assert_eq!(mgr.active_topologies().len().await, 2);

        // Two Mounted events on the bus.
        let mut mounted = 0;
        for _ in 0..4 {
            match timeout(Duration::from_millis(200), rx.recv()).await {
                Ok(Ok(FlowDefinitionEvent::Mounted { .. })) => mounted += 1,
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert_eq!(mounted, 2);
    }

    /// HR-4 (HR5): a flow whose head body references an unknown
    /// kind surfaces as `ResolveFailed` during boot_resume; the
    /// walk continues for the other flows.
    #[tokio::test]
    async fn hr4_boot_resume_resolve_failure_does_not_abort_walk() {
        // Build a manager with a kind registered, publish a flow,
        // then build a *second* manager sharing the store but with
        // an empty kind registry so the head body fails to resolve.
        let store = MemStore::new();
        let kinds_with = Arc::new(NodeKindRegistry::new());
        kinds_with
            .register(AnyKind::arc("com.example.any"))
            .await
            .unwrap();
        let mgr_with = DefinitionManager::new(store.clone(), kinds_with);

        let flow = FlowId::new("examples.boot.bad").unwrap();
        mgr_with
            .publish(
                flow.clone(),
                serde_json::json!({
                    "flow_id": "examples.boot.bad",
                    "nodes": [{"id": "boot.n", "kind": "com.example.any"}],
                    "links": []
                }),
                DefinitionSource::Api,
            )
            .await
            .unwrap();

        // Boot a fresh manager with an empty registry.
        let kinds_empty = Arc::new(NodeKindRegistry::new());
        let mgr_boot = DefinitionManager::new(store, kinds_empty);
        let mut rx = mgr_boot.subscribe();
        let report = mgr_boot.boot_resume().await.expect("boot_resume");
        assert_eq!(report.mounted, 0);
        assert_eq!(report.failed, 1);
        assert_eq!(mgr_boot.active_topologies().len().await, 0);

        // ResolveFailed event on the bus.
        let mut saw_failed = false;
        for _ in 0..2 {
            if let Ok(Ok(FlowDefinitionEvent::ResolveFailed { .. })) =
                timeout(Duration::from_millis(100), rx.recv()).await
            {
                saw_failed = true;
                break;
            }
        }
        assert!(saw_failed, "ResolveFailed must be emitted for unresolvable head");
    }

    // -------- HR-6 tests --------

    /// HR-6 / HR8 first paragraph: registering a previously-missing
    /// kind remounts every flow that was stuck in `ResolveFailed`
    /// because of it.
    #[tokio::test]
    async fn hr6_on_kind_registered_remounts_previously_failed_flows() {
        // Build a manager whose registry is missing the kind; seed
        // a failed flow via boot_resume on a shared store that
        // already has a published head referencing the kind.
        let store = MemStore::new();
        let kinds_with = Arc::new(NodeKindRegistry::new());
        kinds_with
            .register(AnyKind::arc("com.example.any"))
            .await
            .unwrap();
        let mgr_with = DefinitionManager::new(store.clone(), kinds_with);
        let flow = FlowId::new("examples.hr6.remount").unwrap();
        mgr_with
            .publish(
                flow.clone(),
                serde_json::json!({
                    "flow_id": "examples.hr6.remount",
                    "nodes": [{"id": "boot.n", "kind": "com.example.any"}],
                    "links": []
                }),
                DefinitionSource::Api,
            )
            .await
            .unwrap();

        // Fresh manager, empty registry -> boot_resume records
        // the flow as failed.
        let kinds_empty = Arc::new(NodeKindRegistry::new());
        let mgr = DefinitionManager::new(store, kinds_empty.clone());
        let report = mgr.boot_resume().await.unwrap();
        assert_eq!(report.failed, 1);
        assert_eq!(mgr.failed_flows().await.len(), 1);
        assert_eq!(mgr.active_topologies().len().await, 0);

        // Register the kind, then notify the manager.
        kinds_empty
            .register(AnyKind::arc("com.example.any"))
            .await
            .unwrap();
        let mut rx = mgr.subscribe();
        let kind = KindId::new("com.example.any").unwrap();
        let remounted = mgr.on_kind_registered(&kind).await;
        assert_eq!(remounted, 1);
        assert_eq!(mgr.active_topologies().len().await, 1);
        assert!(mgr.failed_flows().await.is_empty());

        // Mounted event on the bus.
        let evt = timeout(Duration::from_millis(200), rx.recv()).await;
        assert!(matches!(evt, Ok(Ok(FlowDefinitionEvent::Mounted { .. }))));
    }

    /// HR-6 / HR8 second paragraph: deregistering a kind revokes
    /// every active topology that references it, transitions the
    /// flow to `ResolveFailed`, and emits `KindRevoked`.
    #[tokio::test]
    async fn hr6_on_kind_deregistered_revokes_active_topology() {
        let (mgr, _store, _graph) = build_manager_with_graph().await;
        let flow = FlowId::new("examples.hr6.revoke").unwrap();
        mgr.publish(
            flow.clone(),
            serde_json::json!({
                "flow_id": "examples.hr6.revoke",
                "apply_policy": "drain",
                "nodes": [{"id": "boot.n", "kind": "com.example.any"}],
                "links": []
            }),
            DefinitionSource::Api,
        )
        .await
        .unwrap();
        assert_eq!(mgr.active_topologies().len().await, 1);

        let mut rx = mgr.subscribe();
        let kind = KindId::new("com.example.any").unwrap();
        let revoked = mgr.on_kind_deregistered(&kind).await;
        assert_eq!(revoked, 1);
        assert_eq!(mgr.active_topologies().len().await, 0);
        assert_eq!(mgr.failed_flows().await.len(), 1);

        let mut saw_revoked = false;
        let mut saw_failed = false;
        for _ in 0..4 {
            match timeout(Duration::from_millis(200), rx.recv()).await {
                Ok(Ok(FlowDefinitionEvent::KindRevoked {
                    kind: k,
                    apply_policy,
                    ..
                })) => {
                    assert_eq!(k, "com.example.any");
                    assert_eq!(apply_policy, ApplyPolicy::Drain);
                    saw_revoked = true;
                }
                Ok(Ok(FlowDefinitionEvent::ResolveFailed { .. })) => saw_failed = true,
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(saw_revoked, "KindRevoked must be emitted");
        assert!(saw_failed, "ResolveFailed must follow revocation");
    }

    /// HR-6: `restart` policy on revocation cancels every
    /// in-flight `RunCancel` registered under the flow's head.
    #[tokio::test]
    async fn hr6_deregister_with_restart_cancels_in_flight_runs() {
        use starter_flow_spi::Cancel;
        let (mgr, _store, _graph) = build_manager_with_graph().await;
        let flow = FlowId::new("examples.hr6.restart").unwrap();
        mgr.publish(
            flow.clone(),
            serde_json::json!({
                "flow_id": "examples.hr6.restart",
                "apply_policy": "restart",
                "nodes": [{"id": "boot.n", "kind": "com.example.any"}],
                "links": []
            }),
            DefinitionSource::Api,
        )
        .await
        .unwrap();
        let head = mgr.store().head(flow.clone()).await.unwrap().unwrap();

        let cancel = Arc::new(RunCancel::new());
        let _guard = mgr.register_run(flow.clone(), head, Arc::clone(&cancel));

        let kind = KindId::new("com.example.any").unwrap();
        let revoked = mgr.on_kind_deregistered(&kind).await;
        assert_eq!(revoked, 1);
        assert!(cancel.is_cancelled());
    }

    /// HR-6: deregistering a kind not referenced by any active
    /// topology is a no-op (no events, no failed entries).
    #[tokio::test]
    async fn hr6_deregister_unreferenced_kind_is_noop() {
        let (mgr, _store, _graph) = build_manager_with_graph().await;
        let flow = FlowId::new("examples.hr6.noop").unwrap();
        mgr.publish(
            flow,
            serde_json::json!({
                "flow_id": "examples.hr6.noop",
                "nodes": [{"id": "boot.n", "kind": "com.example.any"}],
                "links": []
            }),
            DefinitionSource::Api,
        )
        .await
        .unwrap();
        let other = KindId::new("com.example.unused").unwrap();
        let revoked = mgr.on_kind_deregistered(&other).await;
        assert_eq!(revoked, 0);
        assert_eq!(mgr.active_topologies().len().await, 1);
        assert!(mgr.failed_flows().await.is_empty());
    }

    /// HR-6: a successful republish supersedes a prior failure;
    /// the flow no longer appears in `failed_flows()`.
    #[tokio::test]
    async fn hr6_successful_publish_clears_failed_entry() {
        // First, drive a flow into the failed map via revocation.
        let (mgr, _store, _graph) = build_manager_with_graph().await;
        let flow = FlowId::new("examples.hr6.republish").unwrap();
        mgr.publish(
            flow.clone(),
            serde_json::json!({
                "flow_id": "examples.hr6.republish",
                "nodes": [{"id": "boot.n", "kind": "com.example.any"}],
                "links": []
            }),
            DefinitionSource::Api,
        )
        .await
        .unwrap();
        let kind = KindId::new("com.example.any").unwrap();
        let revoked = mgr.on_kind_deregistered(&kind).await;
        assert_eq!(revoked, 1);
        assert_eq!(mgr.failed_flows().await.len(), 1);

        // Republish — succeeds because the registry still holds
        // the kind from build_manager_with_graph (deregister was
        // only at the manager-walk level above).
        mgr.publish(
            flow.clone(),
            serde_json::json!({
                "flow_id": "examples.hr6.republish",
                "nodes": [
                    {"id": "boot.n", "kind": "com.example.any"},
                    {"id": "boot.m", "kind": "com.example.any"}
                ],
                "links": []
            }),
            DefinitionSource::Api,
        )
        .await
        .unwrap();
        assert!(mgr.failed_flows().await.is_empty());
    }

    /// HR-6 engine wiring: `Engine::register_kind` /
    /// `Engine::deregister_kind` route through the attached
    /// `DefinitionManager` so a host that never touches the
    /// registry directly still gets remount-on-register and
    /// revoke-on-deregister behaviour.
    #[tokio::test]
    async fn hr6_engine_register_and_deregister_kind_fires_walks() {
        use crate::engine::Engine;
        use crate::graph::InMemoryGraphStore;

        let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let store = MemStore::new();
        // Empty registry: any flow referencing `com.example.any`
        // will resolve-fail until the kind is registered.
        let kinds = Arc::new(NodeKindRegistry::new());
        let mgr = Arc::new(DefinitionManager::new(store.clone(), Arc::clone(&kinds)));
        // Share the registry so engine.register_kind hits the same
        // Arc the manager uses for remount resolution.
        let engine = Engine::new(graph)
            .with_node_kinds(Arc::clone(&kinds))
            .with_definition_manager(Arc::clone(&mgr));

        // Seed a flow whose head references the not-yet-registered
        // kind via a separate manager with a registered copy.
        let kinds_seed = Arc::new(NodeKindRegistry::new());
        kinds_seed
            .register(AnyKind::arc("com.example.any"))
            .await
            .unwrap();
        let mgr_seed = DefinitionManager::new(store, kinds_seed);
        let flow = FlowId::new("examples.hr6.engine").unwrap();
        mgr_seed
            .publish(
                flow.clone(),
                serde_json::json!({
                    "flow_id": "examples.hr6.engine",
                    "nodes": [{"id": "boot.n", "kind": "com.example.any"}],
                    "links": []
                }),
                DefinitionSource::Api,
            )
            .await
            .unwrap();

        // Boot the real engine's manager into ResolveFailed for the flow.
        let report = mgr.boot_resume().await.unwrap();
        assert_eq!(report.failed, 1);

        // engine.register_kind must fire the remount walk.
        let mut rx = mgr.subscribe();
        engine
            .register_kind(AnyKind::arc("com.example.any"))
            .await
            .unwrap();
        let evt = timeout(Duration::from_millis(200), rx.recv()).await;
        assert!(
            matches!(evt, Ok(Ok(FlowDefinitionEvent::Mounted { .. }))),
            "Mounted must be emitted by engine.register_kind, got {evt:?}"
        );
        assert_eq!(mgr.active_topologies().len().await, 1);

        // engine.deregister_kind must fire the revoke walk.
        let mut rx2 = mgr.subscribe();
        let kind = KindId::new("com.example.any").unwrap();
        engine.deregister_kind(&kind).await.unwrap();
        let mut saw_revoked = false;
        for _ in 0..4 {
            match timeout(Duration::from_millis(200), rx2.recv()).await {
                Ok(Ok(FlowDefinitionEvent::KindRevoked { .. })) => {
                    saw_revoked = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(saw_revoked, "KindRevoked must be emitted by engine.deregister_kind");
        assert_eq!(mgr.active_topologies().len().await, 0);
    }
}
