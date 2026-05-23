//! `FlowRegistry` — pre-resolved flows ready to wrap as
//! `FlowAsTool::from_registry`.
//!
//! See `docs/design/starter-changes/README.md` Phase 2b U3 for the
//! gap and shape. The registry holds, per `(FlowId, FlowRevisionId)`:
//!
//! - the parsed [`FlowBody`] (the canonical typed body the publish
//!   chokepoint operates on);
//! - the resolved [`Arc<FlowTopology>`] (computed once at register
//!   time against the caller-supplied [`NodeKindRegistry`]);
//! - the terminal slots a [`crate::FlowAsTool`] reads back at run
//!   end;
//! - the explicit input / output JSON schemas D-F3.4 forbids
//!   deriving from the body;
//! - the tool-surface metadata (`tool_id`, `name`, `description`)
//!   plus the seed-slot the default seed adapter writes the JSON
//!   input to.
//!
//! Adapters are not derived from the body — D-F3.4 (no derivation
//! from the flow revision) extends to the imperative seed and
//! output sides. [`FlowRegistration::with_default_adapters`] gives
//! the common "single seed slot, single terminal slot" shape every
//! `tools/call` boundary needs in one line; callers wanting
//! per-flow shaping pass their own [`crate::SeedAdapter`] /
//! [`crate::OutputAdapter`] through the lower-level
//! [`FlowRegistration::with_adapters`] entrypoint.
//!
//! The registry is keyed by `(FlowId, FlowRevisionId)` and guarded
//! by `tokio::sync::RwLock` for the same reason
//! `starter-flow::registry::FlowRegistry` is: the
//! `FlowAsTool::from_registry` read path is vastly more common
//! than registration writes. Re-registering the same
//! `(flow_id, revision)` pair is refused with
//! [`FlowRegistryError::DuplicateRevision`]; revisions are immutable.

use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::RwLock;

use starter_flow::definition::body::FlowBody;
use starter_flow::definition::TopologyResolverError;
use starter_flow::propagator::FlowTopology;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::node::{KindId, SlotRef};

use crate::{OutputAdapter, SeedAdapter};

pub mod register;
pub mod resolve;

pub use register::FlowRegistration;

/// Errors raised by [`FlowRegistry`] register / resolve.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FlowRegistryError {
    /// The same `(flow_id, revision_id)` was registered twice.
    /// Revisions are immutable; re-registration is refused.
    #[error("flow {flow} revision {revision} is already registered")]
    DuplicateRevision {
        /// The flow id targeted by the duplicate.
        flow: FlowId,
        /// The duplicate revision id.
        revision: FlowRevisionId,
    },

    /// The lookup [`FlowRegistry::resolve`] performed found no
    /// entry under `(flow_id, revision_id)`.
    #[error("no flow registered under {flow} revision {revision}")]
    NotFound {
        /// The flow id the caller asked for.
        flow: FlowId,
        /// The revision id the caller asked for.
        revision: FlowRevisionId,
    },

    /// The YAML loader failed to deserialise the file into the
    /// typed [`FlowBody`] shape.
    #[error("flow YAML invalid at {path}: {detail}")]
    YamlShape {
        /// Source path or label for diagnostics.
        path: String,
        /// Human-readable description of the deserialise failure.
        detail: String,
    },

    /// The topology resolver refused the body at register time
    /// (unknown kind, bad settings, malformed link, ...). Carries
    /// the structured resolver error verbatim.
    #[error("topology resolve failed for {flow} revision {revision}: {error}")]
    Resolve {
        /// The flow id the resolver was working on.
        flow: FlowId,
        /// The revision id the resolver was working on.
        revision: FlowRevisionId,
        /// The structured resolver error.
        #[source]
        error: TopologyResolverError,
    },

    /// A terminal slot named at registration time references a
    /// node id that is not declared in the flow body. Caught at
    /// register time so [`FlowAsTool::from_registry`] never panics
    /// reading an unknown slot back.
    #[error("terminal slot {slot:?} references node `{node}` not declared in flow body")]
    UnknownTerminalNode {
        /// The offending slot.
        slot: SlotRef,
        /// The node id that is missing from the body.
        node: String,
    },
}

/// One entry in a [`FlowRegistry`] — everything
/// [`crate::FlowAsTool::from_registry`] needs to wrap the flow as
/// an MCP-callable / CLI-callable / REST-callable tool without
/// hand-rolling per-flow glue.
#[non_exhaustive]
pub struct RegisteredFlow {
    /// The flow id.
    pub flow_id: FlowId,
    /// The pinned revision.
    pub revision: FlowRevisionId,
    /// The parsed typed body.
    pub body: FlowBody,
    /// Topology pre-resolved against the registration-time
    /// [`NodeKindRegistry`].
    pub topology: Arc<FlowTopology>,
    /// The slots read back at the end of a successful run; passed
    /// straight to [`crate::FlowAsToolBuilder::terminal_slots`].
    pub terminal_slots: Vec<SlotRef>,
    /// Explicit input JSON-schema (D-F3.4).
    pub input_schema: serde_json::Value,
    /// Explicit output JSON-schema (D-F3.4).
    pub output_schema: serde_json::Value,
    /// The reverse-DNS tool id (R10).
    pub tool_id: KindId,
    /// Tool name surfaced in `tools/list` and CLI subcommand
    /// derivation.
    pub name: String,
    /// One-sentence human description.
    pub description: String,
    /// Imperative seed adapter (D-F3.4).
    pub seed_adapter: SeedAdapter,
    /// Imperative output adapter (D-F3.4).
    pub output_adapter: OutputAdapter,
}

/// Bundle returned by [`FlowRegistry::resolve`].
///
/// Per the U3 ledger entry the resolve surface returns the
/// `(topology, terminal slots)` pair; the registered tool
/// descriptor (schemas + name + adapters) rides along so callers
/// that wire something other than [`crate::FlowAsTool`] (a future
/// gRPC surface, a CLI subcommand factory, ...) can build their
/// own wrapper without going through `from_registry`.
#[non_exhaustive]
pub struct ResolvedFlow {
    /// Resolved propagator topology.
    pub topology: Arc<FlowTopology>,
    /// Terminal slots read back at run end.
    pub terminal_slots: Vec<SlotRef>,
    /// Registered tool id (R10).
    pub tool_id: KindId,
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// Input JSON-schema (D-F3.4).
    pub input_schema: serde_json::Value,
    /// Output JSON-schema (D-F3.4).
    pub output_schema: serde_json::Value,
    /// Seed adapter as registered.
    pub seed_adapter: SeedAdapter,
    /// Output adapter as registered.
    pub output_adapter: OutputAdapter,
}

/// Registry of flow definitions paired with the tool-surface
/// metadata `FlowAsTool::from_registry` needs.
///
/// Distinct from `starter-flow::registry::FlowRegistry`: that one
/// is the Phase-3 SQLite-backed durable definition store the
/// engine owns; this one is the per-process "things I want to
/// expose as tools" map that `starter-mcp` (and future
/// `starter-cli` / `starter-grpc` tool surfaces) load at boot.
/// The two compose: a host can register every flow from the
/// durable store into this registry, or hand-pick a subset.
pub struct FlowRegistry {
    inner: RwLock<HashMap<(FlowId, FlowRevisionId), Arc<RegisteredFlow>>>,
}

impl Default for FlowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Count the registered flows (test / inspector convenience).
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Whether the registry is empty.
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }

    /// Lower-level lookup returning the cached
    /// [`RegisteredFlow`] under `(flow_id, revision)`.
    /// `FlowAsTool::from_registry` calls this internally; exposed
    /// for hosts that want to introspect registrations.
    pub async fn lookup(
        &self,
        flow_id: &FlowId,
        revision: &FlowRevisionId,
    ) -> Option<Arc<RegisteredFlow>> {
        self.inner
            .read()
            .await
            .get(&(flow_id.clone(), *revision))
            .cloned()
    }
}

/// Internal helper — splits the write-lock body out of
/// `register.rs` so the public `register` / `register_yaml`
/// methods stay readable.
pub(crate) async fn insert_registered(
    registry: &FlowRegistry,
    entry: RegisteredFlow,
) -> Result<Arc<RegisteredFlow>, FlowRegistryError> {
    let mut map = registry.inner.write().await;
    let key = (entry.flow_id.clone(), entry.revision);
    if map.contains_key(&key) {
        return Err(FlowRegistryError::DuplicateRevision {
            flow: entry.flow_id,
            revision: entry.revision,
        });
    }
    let arc = Arc::new(entry);
    map.insert(key, arc.clone());
    Ok(arc)
}

/// Pre-resolve the topology against the caller-supplied kind
/// registry. Shared by `register` and `register_yaml` so the
/// register-time validation is in one place; kept private to the
/// module so external callers always go through one of the named
/// entry points.
pub(crate) async fn resolve_topology(
    body: &FlowBody,
    flow_id: &FlowId,
    revision: FlowRevisionId,
    kinds: &NodeKindRegistry,
) -> Result<Arc<FlowTopology>, FlowRegistryError> {
    starter_flow::definition::TopologyResolver::resolve_body(body, flow_id, kinds)
        .await
        .map_err(|error| FlowRegistryError::Resolve {
            flow: flow_id.clone(),
            revision,
            error,
        })
}

impl std::fmt::Debug for RegisteredFlow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisteredFlow")
            .field("flow_id", &self.flow_id)
            .field("revision", &self.revision)
            .field("tool_id", &self.tool_id)
            .field("name", &self.name)
            .field("terminal_slots", &self.terminal_slots)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResolvedFlow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedFlow")
            .field("tool_id", &self.tool_id)
            .field("name", &self.name)
            .field("terminal_slots", &self.terminal_slots)
            .finish_non_exhaustive()
    }
}

/// Validate every terminal slot points at a declared node. Done
/// at register time so the run-time read-back in `FlowAsTool`
/// can assume the slot's node exists.
pub(crate) fn check_terminal_slots(
    body: &FlowBody,
    terminals: &[SlotRef],
) -> Result<(), FlowRegistryError> {
    for slot in terminals {
        if !body.nodes.iter().any(|n| n.id == slot.node) {
            return Err(FlowRegistryError::UnknownTerminalNode {
                slot: slot.clone(),
                node: slot.node.to_string(),
            });
        }
    }
    Ok(())
}
