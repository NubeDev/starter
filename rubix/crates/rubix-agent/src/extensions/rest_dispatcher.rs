//! Composite REST dispatcher — routes per `RuntimeKind`.
//!
//! `starter-ext-server` ships two `RestDispatcher` impls:
//!
//! - [`BuiltinRestDispatcher`] for in-process extensions.
//! - [`ProcessRestDispatcher`] for child-process extensions over the
//!   supervisor's JSON-RPC stdio.
//!
//! The rubix agent runs both flavours side by side, so the single
//! `Arc<dyn RestDispatcher>` handed to `rest_router` has to pick the
//! right backend per call. This composite inspects the sealed
//! `ExtensionRegistry` for the calling extension's runtime kind and
//! forwards to the matching inner dispatcher.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use starter_ext_host::ExtensionRegistry;
use starter_ext_server::{
    BuiltinRestDispatcher, DispatchError, ProcessRestDispatcher, RestDispatcher, StreamResponse,
};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::{ExtensionId, RuntimeKind};

/// Per-call timeout for process-flavour REST dispatch. Matches the
/// MCP-side `EXTENSION_TOOL_REQUEST_TIMEOUT` so an operator tuning
/// extension timeouts in one surface gets the same behaviour on the
/// other.
pub const EXTENSION_REST_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Routes REST dispatch to either the builtin or the process
/// dispatcher based on the extension's manifest runtime kind.
///
/// Construction order matters: callers pass already-constructed inner
/// dispatchers so the capability factory (builtin) and supervisor
/// handle map (process) stay owned by the boot module and outlive the
/// router.
pub struct CompositeRestDispatcher {
    registry: Arc<ExtensionRegistry>,
    builtin: Arc<BuiltinRestDispatcher>,
    process: Arc<ProcessRestDispatcher>,
}

impl CompositeRestDispatcher {
    /// Build a composite dispatcher from its parts.
    pub fn new(
        registry: Arc<ExtensionRegistry>,
        builtin: Arc<BuiltinRestDispatcher>,
        process: Arc<ProcessRestDispatcher>,
    ) -> Self {
        Self {
            registry,
            builtin,
            process,
        }
    }

    fn pick(&self, extension: &ExtensionId) -> Result<&dyn RestDispatcher, DispatchError> {
        let record = self.registry.get(extension).ok_or_else(|| {
            DispatchError::NotFound(format!("extension {:?}", extension.as_str()))
        })?;
        let manifest = record
            .manifest
            .as_ref()
            .ok_or_else(|| DispatchError::NotFound("manifest missing".into()))?;
        match manifest.runtime.kind {
            RuntimeKind::Builtin => Ok(self.builtin.as_ref()),
            RuntimeKind::Process => Ok(self.process.as_ref()),
            other => Err(DispatchError::NotWired(format!(
                "REST dispatch for runtime kind {other:?} not wired yet"
            ))),
        }
    }
}

#[async_trait]
impl RestDispatcher for CompositeRestDispatcher {
    async fn dispatch(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        caller: Option<CallerIdentity>,
    ) -> Result<serde_json::Value, DispatchError> {
        self.pick(extension)?
            .dispatch(extension, contribute_id, input, caller)
            .await
    }

    async fn dispatch_stream(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        caller: Option<CallerIdentity>,
    ) -> Result<StreamResponse, DispatchError> {
        self.pick(extension)?
            .dispatch_stream(extension, contribute_id, input, caller)
            .await
    }
}
