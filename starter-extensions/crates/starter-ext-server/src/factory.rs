//! Per-record supervisor factory.
//!
//! Enabling a previously-disabled process-flavour extension has to spawn
//! a fresh supervisor against its [`ExtensionRecord`]. That work is
//! pluggable so consumers can wrap it in their own tracing / metrics /
//! launch policy without us reaching into the supervisor's
//! configuration knobs.
//!
//! Builtin-flavour records have no supervisor to spawn — the factory
//! returns `Ok(None)` and the route only flips the persisted state.

use std::sync::Arc;

use async_trait::async_trait;
use starter_ext_host::ExtensionRecord;
use starter_ext_spi::RuntimeKind;
use starter_ext_supervisor::{Supervisor, SupervisorHandle};

/// Spawning the supervisor for one record. Implementations are
/// `Send + Sync + 'static`; called from the `enable` admin handler.
#[async_trait]
pub trait SupervisorFactory: Send + Sync + 'static {
    /// Spawn (or refuse) the supervisor for `record`.
    ///
    /// - Process-flavour: return `Ok(Some(handle))`.
    /// - Builtin/Wasm-flavour: return `Ok(None)` — no supervisor needed.
    /// - On spawn failure: return `Err`. The admin endpoint surfaces
    ///   this as HTTP 500 and records the failure in tracing.
    async fn spawn(
        &self,
        record: &ExtensionRecord,
    ) -> Result<Option<SupervisorHandle>, SupervisorFactoryError>;
}

/// Error from [`SupervisorFactory::spawn`]. Wraps the supervisor's own
/// error type behind a single `String` so the admin endpoint doesn't
/// have to thread `starter_ext_spi::Error` through HTTP responses.
#[derive(Debug, thiserror::Error)]
#[error("supervisor factory error: {0}")]
pub struct SupervisorFactoryError(pub String);

impl SupervisorFactoryError {
    /// Construct from any displayable type.
    pub fn new(msg: impl std::fmt::Display) -> Self {
        Self(msg.to_string())
    }
}

/// Default factory: calls [`Supervisor::start`] for process records;
/// returns `Ok(None)` for builtin/wasm records.
///
/// Suitable for v0.1 consumers that don't need to customise the spawn
/// step. The factory is a zero-sized type so a single
/// `Arc<DefaultSupervisorFactory>` is fine to share.
#[derive(Debug, Default)]
pub struct DefaultSupervisorFactory;

#[async_trait]
impl SupervisorFactory for DefaultSupervisorFactory {
    async fn spawn(
        &self,
        record: &ExtensionRecord,
    ) -> Result<Option<SupervisorHandle>, SupervisorFactoryError> {
        let manifest = match record.manifest.as_ref() {
            Some(m) => m,
            None => return Ok(None), // failed record; nothing to spawn
        };
        match manifest.runtime.kind {
            RuntimeKind::Process => Supervisor::start(record)
                .map(Some)
                .map_err(|e| SupervisorFactoryError::new(e)),
            RuntimeKind::Builtin | RuntimeKind::Wasm => Ok(None),
        }
    }
}

/// Convenience: `Arc<dyn SupervisorFactory>` used in `ExtensionAdmin`.
pub(crate) type DynFactory = Arc<dyn SupervisorFactory>;
