//! Hot-reload definition layer (`DOCS/flow/scope/hot-reload.md`).
//!
//! This module owns the HR1 publish chokepoint
//! ([`DefinitionManager::publish`]), the HR5 resolver
//! ([`TopologyResolver::resolve`] — body JSON → runnable
//! [`crate::propagator::FlowTopology`]), and the typed body shape
//! the engine and CLI both parse the on-disk / on-wire JSON into
//! ([`body::FlowBody`]).
//!
//! Phase HR-1 ships:
//!
//! - [`body::FlowBody`] / [`body::NodeDecl`] / [`body::LinkDecl`] —
//!   the typed JSON shape every editor surface produces.
//! - [`canonical::canonicalise`] — RFC 8785 (JCS) canonicalisation;
//!   the input to the blake3 hash HR1 uses for its idempotent
//!   short-circuit.
//! - [`canonical::body_hash`] — `blake3` over canonical bytes.
//! - [`resolver::TopologyResolver`] — pure projection from a
//!   [`FlowRevision`](starter_flow_spi::flow::FlowRevision) body to
//!   an `Arc<FlowTopology>`. Walks every node's
//!   [`NodeBehavior::validate_settings`](starter_flow_spi::node::NodeBehavior::validate_settings)
//!   and refuses anything that wouldn't run.
//! - [`manager::DefinitionManager`] — the single-writer publish
//!   chokepoint HR1 names. Validates, canonicalises, hashes, looks
//!   up the head's hash, short-circuits or writes a fresh
//!   [`FlowRevision`](starter_flow_spi::flow::FlowRevision), and
//!   emits a [`FlowDefinitionEvent`](starter_flow_spi::definition::FlowDefinitionEvent)
//!   on the engine's definition bus.
//!
//! Phases HR-2..HR-6 (the `ActiveTopology` swap, classifier,
//! observability spans/metrics, boot resume, file-watch adapter, and
//! the kind revoke walk) layer on top of these primitives without
//! changing the chokepoint shape.

pub mod active;
pub mod body;
pub mod canonical;
pub mod classifier;
pub mod manager;
pub mod metrics;
pub mod resolver;
pub mod runs;

pub use active::ActiveTopologies;
pub use classifier::{EditKind, classify};
pub use manager::{BootResumeReport, DefinitionManager, PublishError, PublishOutcome};
pub use metrics::{DefinitionMetrics, DefinitionMetricsCell};
pub use resolver::{TopologyResolver, TopologyResolverError};
pub use runs::{RunRegistration, RunRegistry};
