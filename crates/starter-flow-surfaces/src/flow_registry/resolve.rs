//! `FlowRegistry::resolve` + `FlowAsTool::from_registry`.
//!
//! `resolve` is the O(1) lookup that returns the pre-computed
//! `(topology, terminal_slots, schemas, adapters, tool metadata)`
//! bundle for a `(flow_id, revision)` pair. `from_registry` is
//! the one-call convenience that plugs the bundle plus the
//! caller-supplied [`Engine`] handle into the existing
//! [`crate::FlowAsToolBuilder`] — so consumers no longer
//! hand-roll the ~50 LOC of glue per flow the U3 ledger entry
//! flagged.
//!
//! See `docs/design/starter-changes/README.md` Phase 2b U3.

use std::sync::Arc;

use starter_flow::engine::Engine;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};

use crate::{FlowAsTool, FlowAsToolBuildError};

use super::{FlowRegistry, FlowRegistryError, ResolvedFlow};

impl FlowRegistry {
    /// Resolve a registered `(flow_id, revision)` pair into the
    /// `(topology, terminals, schemas, adapters, tool metadata)`
    /// bundle [`FlowAsTool::from_registry`] wires into the
    /// builder.
    ///
    /// Returns [`FlowRegistryError::NotFound`] when nothing has
    /// been registered under the pair. All other failure modes
    /// (resolver errors, malformed terminal slots, ...) surface at
    /// register time — `resolve` itself is infallible past the
    /// lookup.
    pub async fn resolve(
        &self,
        flow_id: &FlowId,
        revision: &FlowRevisionId,
    ) -> Result<ResolvedFlow, FlowRegistryError> {
        let registered =
            self.lookup(flow_id, revision)
                .await
                .ok_or_else(|| FlowRegistryError::NotFound {
                    flow: flow_id.clone(),
                    revision: *revision,
                })?;

        Ok(ResolvedFlow {
            topology: registered.topology.clone(),
            terminal_slots: registered.terminal_slots.clone(),
            tool_id: registered.tool_id.clone(),
            name: registered.name.clone(),
            description: registered.description.clone(),
            input_schema: registered.input_schema.clone(),
            output_schema: registered.output_schema.clone(),
            seed_adapter: registered.seed_adapter.clone(),
            output_adapter: registered.output_adapter.clone(),
        })
    }
}

impl FlowAsTool {
    /// Build a [`FlowAsTool`] from a registered
    /// `(flow_id, revision)` pair on `registry`, wiring the
    /// engine in directly.
    ///
    /// One call replaces the ~50 LOC of hand-rolled per-flow glue
    /// the U3 ledger entry counted in
    /// `crates/smoke-tests/tests/flow_via_mcp.rs`:
    ///
    /// ```ignore
    /// let tool = FlowAsTool::from_registry(
    ///     &registry,
    ///     &FlowId::new("com.rubix.scheduled-system-check").unwrap(),
    ///     &revision_id,
    ///     engine,
    /// ).await?;
    /// ```
    ///
    /// Errors:
    ///
    /// - [`FromRegistryError::NotRegistered`] if no flow is
    ///   registered under the pair;
    /// - [`FromRegistryError::Build`] if the underlying
    ///   [`crate::FlowAsToolBuilder`] rejects the bundle
    ///   (impossible in practice because the registry validates
    ///   every required field at register time, but surfaced so
    ///   the call site does not need an `.unwrap()`).
    pub async fn from_registry(
        registry: &FlowRegistry,
        flow_id: &FlowId,
        revision: &FlowRevisionId,
        engine: Arc<Engine>,
    ) -> Result<Self, FromRegistryError> {
        let resolved = registry
            .resolve(flow_id, revision)
            .await
            .map_err(FromRegistryError::NotRegistered)?;

        Self::builder()
            .flow_id(flow_id.clone())
            .revision(*revision)
            .topology(resolved.topology)
            .terminal_slots(resolved.terminal_slots)
            .engine(engine)
            .tool_id(resolved.tool_id)
            .name(resolved.name)
            .description(resolved.description)
            .input_schema(resolved.input_schema)
            .output_schema(resolved.output_schema)
            .seed_adapter(resolved.seed_adapter)
            .output_adapter(resolved.output_adapter)
            .build()
            .map_err(FromRegistryError::Build)
    }
}

/// Error returned by [`FlowAsTool::from_registry`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FromRegistryError {
    /// No flow registered under the requested
    /// `(flow_id, revision)`.
    #[error(transparent)]
    NotRegistered(#[from] FlowRegistryError),
    /// The builder rejected the bundle. Cannot happen for any
    /// registration that came through
    /// [`FlowRegistry::register`] — surfaced for defence in
    /// depth.
    #[error("FlowAsTool builder rejected registered flow: {0}")]
    Build(FlowAsToolBuildError),
}
