//! Node-level contracts.
//!
//! Per `DOCS/flow/scope/SCOPE.md` R1 (Everything is a Node), R5
//! (node behaviours are stateless — `&self`, never `&mut self`), and
//! R10 (reverse-DNS ids; namespace ownership enforced).

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A behaviour the engine invokes when a node fires.
///
/// SCOPE R5: stateless. `&self`, never `&mut self`. Per-instance state
/// lives in slots, the session/run store, and the secret store — not
/// on the behaviour.
///
/// Phase 1 ships only the trait shape; concrete `NodeCtx`, the
/// `LifecycleEvent` payload, and the propagator that drives both come
/// in later phases. The `on_lifecycle` default is `Ok(())` so most
/// kinds opt out simply by not overriding it.
#[async_trait]
pub trait NodeBehavior: Send + Sync + 'static {
    /// The node-kind id this behaviour implements. The engine reads
    /// this to route invocations from the `NodeKindRegistry`.
    fn kind_id(&self) -> &KindId;

    /// Invoke the node. Reads the input [`SlotMap`], returns the
    /// output [`SlotMap`]. The engine wires the output back through
    /// the single `GraphStore::write_slot` chokepoint (R2).
    async fn invoke(&self, ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError>;

    /// Slot names whose writes wake (invoke) this node.
    ///
    /// Event-driven runtime semantics: writes to any slot in this set
    /// cause the propagator to re-fire the node's [`Self::invoke`].
    /// Combined with [`Self::read_slots`] this set forms the node's
    /// declared input surface — at invoke time the propagator reads
    /// the union of both sets from the graph store and passes them in
    /// as the input [`SlotMap`].
    ///
    /// Default impl returns `&[]`; kinds with no wake inputs are
    /// triggered only by direct activation or by the body-level YAML
    /// `triggers:` additive override (deprecated; here for transitional
    /// compatibility).
    fn trigger_slots(&self) -> &'static [&'static str] {
        &[]
    }

    /// Slot names this node reads but does **not** wake on.
    ///
    /// Event-driven runtime semantics: writes to a `read_slots` slot
    /// update the value in the graph store but do not cause the
    /// propagator to invoke the node. The slot value is still
    /// available in the [`SlotMap`] passed to [`Self::invoke`] at the
    /// next wake (the propagator reads the union of trigger + read
    /// slots from the store before each invocation).
    ///
    /// Canonical use: configuration-style inputs that the body needs
    /// to read but whose changes alone should not drive a fire. The
    /// tool-call kind declares `tool_id` and `input` here — the body
    /// reads them per invocation, but only the upstream `in` slot
    /// wakes the node.
    ///
    /// Default impl returns `&[]`.
    fn read_slots(&self) -> &'static [&'static str] {
        &[]
    }

    /// React to a lifecycle transition (Created → Active → Paused →
    /// Stopped → Removed per R1). The engine drives this; kinds that
    /// hold no per-lifecycle state ignore it.
    #[allow(unused_variables)]
    async fn on_lifecycle(&self, ctx: NodeCtx<'_>, ev: LifecycleEvent) -> Result<(), NodeError> {
        Ok(())
    }

    /// Hook fired after the engine reconciles a new flow revision into
    /// the running topology (see `DOCS/flow/scope/hot-reload.md`). The
    /// engine reports *what* changed via [`EditKind`] so kinds with a
    /// `reset_on_redeploy` policy (the counter is the canonical
    /// example) can clear their persisted state through the
    /// [`crate::state::NodeStateStore`] seam on the ctx.
    ///
    /// Default impl is a no-op so existing kinds compile unchanged.
    #[allow(unused_variables)]
    async fn on_redeploy(&self, ctx: NodeCtx<'_>, edit: EditKind) -> Result<(), NodeError> {
        Ok(())
    }

    /// JSON Schema describing this kind's settings (the typed,
    /// publish-time configuration carried on the node's
    /// `settings:` field in a flow body).
    ///
    /// Per `DOCS/flow/scope/settings.md` S-1 + S-2: the schema is
    /// derived from the kind's `#[derive(JsonSchema)]` `Settings`
    /// struct via `schemars::schema_for!(...)`, stored as a
    /// `LazyLock<RootSchema>`, and returned by reference here. The
    /// default impl returns
    /// [`crate::settings::EMPTY_SCHEMA`] — a kind with no settings
    /// compiles unchanged.
    fn config_schema(&self) -> &'static schemars::schema::RootSchema {
        &crate::settings::EMPTY_SCHEMA
    }

    /// Validate a draft `settings` body against this kind's schema.
    ///
    /// Per `DOCS/flow/scope/settings.md` S-2: the default impl runs
    /// the body through [`crate::settings::default_validate`] using
    /// [`Self::config_schema`]. Kinds with cross-field rules JSON
    /// Schema cannot express (e.g. *"if `auth_kind = bearer` then
    /// `auth_token` is required"*) override this method, call the
    /// default first, and surface domain rules as
    /// [`crate::settings::SettingsError::Domain`].
    ///
    /// Called by `DefinitionManager::publish` (see
    /// `DOCS/flow/scope/hot-reload.md` HR1) before a `FlowRevision`
    /// is written. Runtime `invoke()` keeps its existing checks;
    /// schema is the *publish-time* gate, not a second runtime
    /// mechanism.
    fn validate_settings(
        &self,
        body: &serde_json::Value,
    ) -> Result<(), crate::settings::SettingsError> {
        crate::settings::default_validate(self.config_schema(), body)
    }
}

/// Per-invocation context handed to a [`NodeBehavior`].
///
/// Phase 2 (stage 4 — propagator) populates this with the minimum the
/// propagator needs to call into a node body: the [`RunId`] of the
/// owning run, the [`NodeId`] of the node being invoked, and a borrow
/// of the run's [`Cancel`](crate::Cancel) token (R13). Future stages
/// may add more borrowed fields (e.g. `Principal`, `GraphStore`
/// writer, trace span) — the struct is `#[non_exhaustive]` so adding
/// a field is non-breaking, and the [`Self::new`] constructor keeps
/// engine-internal construction routed through one call site that
/// knows the full set of fields.
#[non_exhaustive]
pub struct NodeCtx<'a> {
    /// The run this invocation belongs to.
    pub run: crate::flow::RunId,
    /// The node being invoked.
    pub node: &'a NodeId,
    /// Cancellation handle for the run (SCOPE R13). Node bodies
    /// `select!` against `cancel.cancelled()` or poll
    /// `cancel.is_cancelled()` to abort promptly.
    pub cancel: &'a dyn crate::Cancel,
    /// Frozen skill selection for the owning run (D-F4.4). Every
    /// `ai-agent` node invocation in a single run sees the same
    /// [`SkillSelection`]; the engine's [`crate::skill::SkillSelector`]
    /// is invoked exactly once per `FlowRunner::start`. Non-ai-agent
    /// kinds ignore this field.
    pub skill: &'a crate::skill::SkillSelection,
    /// Per-node persistent state seam (R5 chokepoint; see
    /// `DOCS/flow/scope/node-state.md`). Node bodies that need durable
    /// per-instance state (the canonical example is the counter node's
    /// tick count) call into this borrow rather than reaching for a
    /// global; tests and bare-engine configurations pass
    /// [`crate::state::NOOP_NODE_STATE_STORE`] which errors on writes
    /// so a missing wiring fails loudly instead of silently swallowing
    /// updates.
    pub state: &'a dyn crate::state::NodeStateStore,
    /// Owning flow id, when known. Required by node bodies that build
    /// a [`crate::state::NodeStateKey`] (e.g. `starter.flow.counter`).
    /// `None` in legacy [`NodeCtx::new`] call sites; populated via
    /// [`NodeCtx::with_flow`].
    pub flow: Option<&'a crate::flow::FlowId>,
}

impl<'a> NodeCtx<'a> {
    /// Construct a [`NodeCtx`]. The propagator is the only in-engine
    /// caller; it builds one of these per `NodeBehavior::invoke` call.
    ///
    /// `state` is the per-node persistent state seam; pass
    /// [`crate::state::NOOP_NODE_STATE_STORE`] in tests and bare-engine
    /// configurations that do not exercise node state.
    pub fn new(
        run: crate::flow::RunId,
        node: &'a NodeId,
        cancel: &'a dyn crate::Cancel,
        skill: &'a crate::skill::SkillSelection,
        state: &'a dyn crate::state::NodeStateStore,
    ) -> Self {
        Self {
            run,
            node,
            cancel,
            skill,
            state,
            flow: None,
        }
    }

    /// Like [`NodeCtx::new`] but with an explicit owning [`crate::flow::FlowId`].
    /// Required for node bodies that build a
    /// [`crate::state::NodeStateKey`] (the counter node is the
    /// canonical first consumer).
    pub fn with_flow(
        flow: &'a crate::flow::FlowId,
        run: crate::flow::RunId,
        node: &'a NodeId,
        cancel: &'a dyn crate::Cancel,
        skill: &'a crate::skill::SkillSelection,
        state: &'a dyn crate::state::NodeStateStore,
    ) -> Self {
        Self {
            run,
            node,
            cancel,
            skill,
            state,
            flow: Some(flow),
        }
    }
}

/// What changed in a hot-reload edit, as reported to
/// [`NodeBehavior::on_redeploy`]. Node kinds with a
/// `reset_on_redeploy` policy use this to decide whether to clear
/// persisted [`crate::state::NodeStateStore`] entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EditKind {
    /// The node's `settings:` payload changed; topology unchanged.
    Settings,
    /// The flow's topology (links, slots, surrounding nodes) changed;
    /// this node's settings unchanged.
    Topology,
    /// Both settings and topology changed in the same reconciliation.
    Both,
}

/// Lifecycle transition the engine notifies a node of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LifecycleEvent {
    /// Node was just registered with the engine.
    Created,
    /// Node moved into the running set (engine `Running`, flow active).
    Activated,
    /// Node was paused (per-flow pause or engine `Pausing`).
    Paused,
    /// Node was stopped (per-flow stop or engine `Stopping`).
    Stopped,
    /// Node was removed from the registry.
    Removed,
}

/// Reverse-DNS node identifier (SCOPE R10).
///
/// Newtype around `String`; validates on construction. The same shape
/// the extensions framework uses for its contribution ids — every
/// flow identifier (node ids, kind ids, flow ids) follows the same
/// rule so namespace-ownership checks compose across both frameworks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NodeId(String);

impl NodeId {
    /// Parse a string as a node id. Returns [`IdError`] if the value
    /// is not a valid reverse-DNS identifier.
    pub fn new(s: impl Into<String>) -> Result<Self, IdError> {
        let s = s.into();
        validate_reverse_dns(&s)?;
        Ok(Self(s))
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for NodeId {
    type Error = IdError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NodeId> for String {
    fn from(value: NodeId) -> Self {
        value.0
    }
}

/// Static metadata describing a node kind for discovery surfaces
/// (catalog browsers, designers, MCP `flow.nodes.list`, CLI `--help`).
///
/// Carries i18n *message-key* strings rather than translated text so a
/// single descriptor serves every locale. Resolution happens at the
/// presentation layer through `starter_spi::i18n::MessageKey` and the
/// `starter-i18n` catalogs at `crates/starter-i18n/catalogs/starter/`.
///
/// Each `*_key` field is the reverse-DNS catalog key for one piece of
/// help text:
///
/// - `label_key` — a short human label (e.g. "Log", "Branch"). Used
///   in palettes and dropdowns.
/// - `summary_key` — a one-line description of what the node does.
///   Used in tooltips and list views.
/// - `help_key` — longer-form help describing inputs, outputs, and
///   common usage. Used in detail panes and `--help` surfaces.
///
/// By convention every built-in node uses keys under
/// `starter.flow.node.<kind>.{label,summary,help}` (e.g.
/// `starter.flow.node.log.label`). Third-party node crates follow the
/// same shape under their own reverse-DNS prefix.
///
/// The struct is `#[non_exhaustive]` so future additions (icon, input
/// slot schemas, category) are non-breaking. Each `*_key` field is a
/// [`Cow<'static, str>`] per R-flow-node-2 (`DOCS/extensions/scope/FLOW-NODES.md`):
/// built-in kinds construct a descriptor as `const`, threading
/// `&'static str` literals through [`Cow::Borrowed`] (zero allocation,
/// embeddable in static slices); extension-contributed kinds construct
/// owned descriptors at load time through [`NodeDescriptor::new_owned`]
/// without ever resorting to `Box::leak`. The widening keeps both call
/// sites source-compatible: every existing [`NodeDescriptor::new`]
/// invocation in `starter-flow-nodes` continues to compile because the
/// const constructor still takes `&'static str` and wraps each argument
/// in `Cow::Borrowed` internally.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NodeDescriptor {
    /// Reverse-DNS kind id (matches the value [`NodeBehavior::kind_id`]
    /// returns). Stored as [`Cow<'static, str>`] so built-in descriptors
    /// stay zero-allocation while extension-contributed descriptors own
    /// the string (R-flow-node-2). Validate via [`KindId::new`] at use
    /// sites that need a typed id.
    pub kind: Cow<'static, str>,
    /// i18n catalog key for the short label.
    pub label_key: Cow<'static, str>,
    /// i18n catalog key for the one-line summary.
    pub summary_key: Cow<'static, str>,
    /// i18n catalog key for the long-form help text.
    pub help_key: Cow<'static, str>,
}

impl NodeDescriptor {
    /// `const` constructor for built-in descriptors. External crates use
    /// this rather than the struct literal because [`NodeDescriptor`] is
    /// `#[non_exhaustive]`; future field additions stay non-breaking
    /// because every existing call site goes through here.
    ///
    /// The widening to [`Cow<'static, str>`] in R-flow-node-2 keeps every
    /// existing built-in call site source-compatible: callers pass
    /// `&'static str` literals exactly as before and this constructor
    /// wraps them in [`Cow::Borrowed`].
    pub const fn new(
        kind: &'static str,
        label_key: &'static str,
        summary_key: &'static str,
        help_key: &'static str,
    ) -> Self {
        Self {
            kind: Cow::Borrowed(kind),
            label_key: Cow::Borrowed(label_key),
            summary_key: Cow::Borrowed(summary_key),
            help_key: Cow::Borrowed(help_key),
        }
    }

    /// Owned constructor for extension-contributed descriptors.
    ///
    /// Extensions read their `block.yaml`, materialise the i18n keys, and
    /// call this — owned strings live on the [`DynamicNodeKindRegistry`]
    /// for as long as the registry does (R-flow-node-2: no `Box::leak`).
    pub fn new_owned(
        kind: impl Into<String>,
        label_key: impl Into<String>,
        summary_key: impl Into<String>,
        help_key: impl Into<String>,
    ) -> Self {
        Self {
            kind: Cow::Owned(kind.into()),
            label_key: Cow::Owned(label_key.into()),
            summary_key: Cow::Owned(summary_key.into()),
            help_key: Cow::Owned(help_key.into()),
        }
    }
}

/// Read-only registry exposing the [`NodeDescriptor`]s the engine
/// knows about.
///
/// Adapters (REST, MCP, CLI) call into this to enumerate available
/// kinds and surface help text — the SCOPE R5 chokepoint pattern the
/// `ToolRegistry` already uses for tools. The host populates the
/// registry at engine-build time and hands it out as
/// `Arc<dyn NodeKindRegistry>`; the surface is read-only thereafter.
pub trait NodeKindRegistry: Send + Sync + 'static {
    /// Look up a descriptor by kind id. Returns `None` if no kind is
    /// registered under that id.
    fn lookup(&self, kind: &KindId) -> Option<&NodeDescriptor>;

    /// Snapshot every registered descriptor. Order is registry-defined
    /// (typically registration order); callers that need a stable sort
    /// should sort by `kind`.
    fn all(&self) -> Vec<&NodeDescriptor>;
}

/// One entry in a [`DynamicNodeKindRegistry`].
///
/// Carries an owned [`NodeDescriptor`] (i18n catalog keys + reverse-DNS
/// kind id) and a factory that materialises an [`Arc<dyn NodeBehavior>`]
/// on demand. The factory shape is deliberately closure-typed rather
/// than `Arc<dyn NodeBehavior>` directly because the extensions adapter
/// (`starter-ext-flow::contributed_node_kinds`) wants to keep the
/// behaviour construction lazy: in stage 1 (slice A) every factory
/// returns the same `UnboundNodeBehavior` instance, in stage 2 (slice B)
/// the factory closes over the `SupervisorHandle` and constructs a
/// `ProcessNodeProxy` per call.
///
/// R-flow-node-2 ground rule: the descriptor's [`Cow<'static, str>`]
/// fields hold owned strings whose lifetime tracks the registry's; no
/// `Box::leak` anywhere.
pub struct DynamicNodeKindEntry {
    descriptor: NodeDescriptor,
    factory: Arc<dyn Fn() -> Arc<dyn NodeBehavior> + Send + Sync + 'static>,
}

impl DynamicNodeKindEntry {
    /// Build an entry from an owned descriptor and a factory closure.
    pub fn new<F>(descriptor: NodeDescriptor, factory: F) -> Self
    where
        F: Fn() -> Arc<dyn NodeBehavior> + Send + Sync + 'static,
    {
        Self {
            descriptor,
            factory: Arc::new(factory),
        }
    }

    /// Borrow the descriptor.
    pub fn descriptor(&self) -> &NodeDescriptor {
        &self.descriptor
    }

    /// Materialise the [`NodeBehavior`] implementation for this kind.
    pub fn behavior(&self) -> Arc<dyn NodeBehavior> {
        (self.factory)()
    }
}

impl fmt::Debug for DynamicNodeKindEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicNodeKindEntry")
            .field("descriptor", &self.descriptor)
            .finish_non_exhaustive()
    }
}

/// In-memory [`NodeKindRegistry`] populated from owned
/// [`DynamicNodeKindEntry`] values.
///
/// Per R-flow-node-6 the dynamic registry's contents are *not* `static`
/// — they live until the registry is dropped by the hot-reload algorithm
/// in slice B. Stage 1 (slice A) ships the registry shape + the
/// composition seam; the reload algorithm lands later.
pub struct DynamicNodeKindRegistry {
    entries: Vec<DynamicNodeKindEntry>,
    by_kind: HashMap<String, usize>,
}

impl DynamicNodeKindRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            by_kind: HashMap::new(),
        }
    }

    /// Build a registry from an iterator of entries. The last entry
    /// under a given kind id wins; collisions are tolerated so a host
    /// can swap an extension's contribution in place during hot reload
    /// without an intermediate empty state.
    pub fn from_entries<I>(entries: I) -> Self
    where
        I: IntoIterator<Item = DynamicNodeKindEntry>,
    {
        let mut reg = Self::new();
        for entry in entries {
            reg.insert(entry);
        }
        reg
    }

    /// Insert (or replace) one entry. Returns the previous entry under
    /// the same kind id, if any.
    pub fn insert(&mut self, entry: DynamicNodeKindEntry) -> Option<DynamicNodeKindEntry> {
        let kind = entry.descriptor.kind.clone().into_owned();
        if let Some(&idx) = self.by_kind.get(&kind) {
            let prev = std::mem::replace(&mut self.entries[idx], entry);
            return Some(prev);
        }
        self.by_kind.insert(kind, self.entries.len());
        self.entries.push(entry);
        None
    }

    /// Look up an entry by kind id.
    pub fn entry(&self, kind: &KindId) -> Option<&DynamicNodeKindEntry> {
        self.by_kind
            .get(kind.as_str())
            .and_then(|&idx| self.entries.get(idx))
    }

    /// Borrow every entry. Order is insertion order; for stable
    /// presentation surfaces, sort by `entry.descriptor().kind`.
    pub fn entries(&self) -> &[DynamicNodeKindEntry] {
        &self.entries
    }

    /// `true` if the registry holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl Default for DynamicNodeKindRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeKindRegistry for DynamicNodeKindRegistry {
    fn lookup(&self, kind: &KindId) -> Option<&NodeDescriptor> {
        self.entry(kind).map(|e| &e.descriptor)
    }

    fn all(&self) -> Vec<&NodeDescriptor> {
        self.entries.iter().map(|e| &e.descriptor).collect()
    }
}

/// Composition of a static built-in registry and a dynamic
/// extension-contributed registry.
///
/// Lookup order is **static first**, then dynamic — built-in kinds
/// always win on a collision so an extension cannot shadow
/// `starter.flow.*` (R-flow-node-3 forbids the manifest from declaring
/// such a kind in the first place; the lookup precedence is a
/// belt-and-braces guard against a registry built outside the normal
/// loader path).
pub struct CompositeNodeKindRegistry {
    static_reg: Arc<dyn NodeKindRegistry>,
    dynamic_reg: Arc<DynamicNodeKindRegistry>,
}

impl CompositeNodeKindRegistry {
    /// Construct a composite from a built-in registry and the
    /// extension-contributed registry.
    pub fn new(
        static_reg: Arc<dyn NodeKindRegistry>,
        dynamic_reg: Arc<DynamicNodeKindRegistry>,
    ) -> Self {
        Self {
            static_reg,
            dynamic_reg,
        }
    }

    /// Borrow the dynamic half. Slice B's
    /// `POST /admin/extensions/reload` swaps this through an
    /// `ArcSwap<CompositeNodeKindRegistry>` (R-flow-node-6).
    pub fn dynamic(&self) -> &Arc<DynamicNodeKindRegistry> {
        &self.dynamic_reg
    }

    /// Borrow the static half.
    pub fn statics(&self) -> &Arc<dyn NodeKindRegistry> {
        &self.static_reg
    }
}

impl NodeKindRegistry for CompositeNodeKindRegistry {
    fn lookup(&self, kind: &KindId) -> Option<&NodeDescriptor> {
        self.static_reg
            .lookup(kind)
            .or_else(|| self.dynamic_reg.lookup(kind))
    }

    fn all(&self) -> Vec<&NodeDescriptor> {
        let mut out = self.static_reg.all();
        out.extend(self.dynamic_reg.all());
        out
    }
}

/// Reverse-DNS node-kind identifier (SCOPE R10).
///
/// Same validation rules as [`NodeId`]; a separate type so the
/// compiler refuses to confuse "which node" with "what kind of node".
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KindId(String);

impl KindId {
    /// Parse a string as a kind id. Returns [`IdError`] if the value
    /// is not a valid reverse-DNS identifier.
    pub fn new(s: impl Into<String>) -> Result<Self, IdError> {
        let s = s.into();
        validate_reverse_dns(&s)?;
        Ok(Self(s))
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KindId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for KindId {
    type Error = IdError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<KindId> for String {
    fn from(value: KindId) -> Self {
        value.0
    }
}

/// Reverse-DNS validation shared by every flow identifier.
///
/// Rules (matched to the extensions framework's R4):
/// - non-empty
/// - at least one `.` (i.e. two or more dot-separated segments)
/// - each segment is non-empty, starts with `[a-z]`, and contains only
///   `[a-z0-9_-]` thereafter
pub(crate) fn validate_reverse_dns(s: &str) -> Result<(), IdError> {
    if s.is_empty() {
        return Err(IdError::Empty);
    }
    let segments: Vec<&str> = s.split('.').collect();
    if segments.len() < 2 {
        return Err(IdError::NotReverseDns(s.to_owned()));
    }
    for seg in segments {
        if seg.is_empty() {
            return Err(IdError::NotReverseDns(s.to_owned()));
        }
        let mut chars = seg.chars();
        let first = chars.next().expect("non-empty segment");
        if !first.is_ascii_lowercase() {
            return Err(IdError::NotReverseDns(s.to_owned()));
        }
        for c in chars {
            let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-';
            if !ok {
                return Err(IdError::NotReverseDns(s.to_owned()));
            }
        }
    }
    Ok(())
}

/// Reverse-DNS id validation failure.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum IdError {
    /// Id string was empty.
    #[error("id may not be empty")]
    Empty,
    /// Id was not a valid reverse-DNS identifier per the rules in
    /// [`validate_reverse_dns`].
    #[error("not a valid reverse-DNS id: {0}")]
    NotReverseDns(String),
}

/// A reference to a specific slot on a specific node.
///
/// Per SCOPE R2: slots are the only I/O surface. Every wire — REST,
/// CLI, propagation, replay — names the destination as a [`SlotRef`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SlotRef {
    /// The node that owns the slot.
    pub node: NodeId,
    /// The slot's name within the node.
    pub slot: String,
}

impl SlotRef {
    /// Construct a [`SlotRef`].
    pub fn new(node: NodeId, slot: impl Into<String>) -> Self {
        Self {
            node,
            slot: slot.into(),
        }
    }
}

/// Typed value carried on a slot.
///
/// Per SCOPE R2: slot wire shape is Node-RED compatible — a message
/// envelope with `payload`, `topic`, and arbitrary custom fields. The
/// envelope structure itself lives in the engine; this enum is the
/// type of each individual slot value (the `payload`, the `topic`, the
/// custom fields).
///
/// `#[non_exhaustive]` so future additions (e.g. a typed array variant,
/// timestamps) are non-breaking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SlotValue {
    /// Null / absent.
    Null,
    /// Boolean.
    Bool(bool),
    /// Signed 64-bit integer.
    Int(i64),
    /// 64-bit float.
    Float(f64),
    /// UTF-8 string.
    String(String),
    /// Opaque byte buffer.
    Bytes(Vec<u8>),
    /// Free-form JSON. Escape hatch for kinds whose slot type doesn't
    /// fit the primitives; expected to shrink as the type system grows.
    Json(serde_json::Value),
}

/// Map of slot name → value, used for both node input and node output.
pub type SlotMap = BTreeMap<String, SlotValue>;

/// Error surface a [`NodeBehavior`] returns.
///
/// Phase 1 ships the variants the engine needs to route failures into
/// `FlowEvent::NodeFailed`; richer typed variants land alongside the
/// kinds that need them (R3 `on_failure` policy walks this enum).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NodeError {
    /// Node was invoked with input that did not match its declared
    /// `input_slots` schema.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Node ran but the underlying integration / tool / LLM call
    /// failed. Equivalent to the tools SCOPE's `ToolError::Backend`.
    #[error("backend failure: {0}")]
    Backend(String),

    /// Node was cancelled mid-invocation via its [`Cancel`](crate::Cancel)
    /// token (R13).
    #[error("cancelled")]
    Cancelled,

    /// Node-kind body refused the invocation for a typed, structural
    /// reason (e.g. missing config slot, unregistered provider id,
    /// empty tools intersection). Phase 4 D-F4.5 + D-F4.8: `ai-agent`
    /// surfaces `code = "provider_id_required"`,
    /// `"provider_not_registered"`, `"no_tools_visible"`, and
    /// `"turn_budget_exhausted"` via this variant. The `code` is
    /// machine-readable and stable; `message` is human-friendly and
    /// safe to surface in logs.
    #[error("domain error [{code}]: {message}")]
    Domain {
        /// Short stable machine-readable code.
        code: &'static str,
        /// Human-readable message.
        message: String,
    },

    /// Any other error surfaced by a node-kind body.
    #[error(transparent)]
    Other(#[from] anyhow_compat::Error),
}

/// Internal compatibility module so this contracts crate doesn't pull
/// `anyhow` directly. Future phases swap this for a richer error type
/// once the engine fixes the shape it wants.
#[doc(hidden)]
pub mod anyhow_compat {
    use std::fmt;

    /// Opaque boxed error used by [`super::NodeError::Other`].
    #[derive(Debug)]
    pub struct Error(pub Box<dyn std::error::Error + Send + Sync + 'static>);

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&*self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_dns_accepts_valid() {
        NodeId::new("com.acme.weather").unwrap();
        KindId::new("starter.flow.tool-call").unwrap();
        KindId::new("sys.gate").unwrap();
        NodeId::new("a.b1_c-d.e").unwrap();
    }

    #[test]
    fn descriptor_is_const_constructible() {
        const D: NodeDescriptor = NodeDescriptor::new(
            "starter.flow.log",
            "starter.flow.node.log.label",
            "starter.flow.node.log.summary",
            "starter.flow.node.log.help",
        );
        assert_eq!(D.kind, "starter.flow.log");
        KindId::new(D.kind.clone()).expect("descriptor.kind parses as KindId");
    }

    #[test]
    fn reverse_dns_rejects_invalid() {
        assert!(NodeId::new("").is_err());
        assert!(NodeId::new("nodot").is_err());
        assert!(NodeId::new(".leading").is_err());
        assert!(NodeId::new("trailing.").is_err());
        assert!(NodeId::new("Upper.case").is_err());
        assert!(NodeId::new("starts.1digit").is_err());
        assert!(NodeId::new("two..dots").is_err());
        assert!(NodeId::new("bad char.x").is_err());
    }
}
