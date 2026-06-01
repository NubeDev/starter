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

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use starter_ext_host::ExtensionRecord;
use starter_ext_spi::RuntimeKind;
use starter_ext_supervisor::{Supervisor, SupervisorHandle, SupervisorOptions};

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
/// step. Carries an optional `pidfile_dir` so every supervisor it spawns
/// (including the respawn on `enable` / restart) records its child's
/// process-group id for the boot reaper — set it to the same directory the
/// host passes to `reap_stale_groups` at startup. `None` keeps the prior
/// behaviour (live group-signalling only, no cross-restart pidfile).
#[derive(Debug, Default, Clone)]
pub struct DefaultSupervisorFactory {
    pidfile_dir: Option<PathBuf>,
}

impl DefaultSupervisorFactory {
    /// Factory that writes per-extension pidfiles under `dir` so the boot
    /// reaper can clean groups leaked by a `SIGKILL`ed prior instance.
    pub fn with_pidfile_dir(dir: impl Into<PathBuf>) -> Self {
        Self {
            pidfile_dir: Some(dir.into()),
        }
    }

    fn opts(&self) -> SupervisorOptions {
        SupervisorOptions {
            pidfile_dir: self.pidfile_dir.clone(),
        }
    }
}

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
            RuntimeKind::Process => Supervisor::start_with_opts(
                record,
                Arc::new(starter_ext_supervisor::NotImplementedHandler),
                self.opts(),
            )
            .map(Some)
            .map_err(SupervisorFactoryError::new),
            RuntimeKind::Builtin | RuntimeKind::Wasm => Ok(None),
        }
    }
}

/// Convenience: `Arc<dyn SupervisorFactory>` used in `ExtensionAdmin`.
pub(crate) type DynFactory = Arc<dyn SupervisorFactory>;

/// `SupervisorFactory` that installs a host-provided
/// [`starter_ext_supervisor::HostMethodHandler`] on every
/// process-flavour spawn. Use when the consumer wants real
/// capability-gated host-method bodies (e.g. rubix-agent's
/// `RubixHostMethods` routing `dashboard.read` / `dashboard.write`
/// / `authz.check` into its Row-5 backends).
pub struct WithHostMethodsFactory {
    host_methods: starter_ext_supervisor::SharedHostMethodHandler,
    pidfile_dir: Option<PathBuf>,
}

impl WithHostMethodsFactory {
    /// New factory carrying `host_methods`. Every process-flavour
    /// supervisor it spawns inherits the same `Arc` — cheap to
    /// share, no per-spawn cloning of the underlying handler.
    pub fn new(host_methods: starter_ext_supervisor::SharedHostMethodHandler) -> Self {
        Self {
            host_methods,
            pidfile_dir: None,
        }
    }

    /// Set the directory each spawned supervisor records its child's
    /// process-group id into, for the boot reaper. See
    /// [`DefaultSupervisorFactory::with_pidfile_dir`].
    pub fn with_pidfile_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.pidfile_dir = Some(dir.into());
        self
    }
}

impl std::fmt::Debug for WithHostMethodsFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WithHostMethodsFactory")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl SupervisorFactory for WithHostMethodsFactory {
    async fn spawn(
        &self,
        record: &ExtensionRecord,
    ) -> Result<Option<SupervisorHandle>, SupervisorFactoryError> {
        let manifest = match record.manifest.as_ref() {
            Some(m) => m,
            None => return Ok(None),
        };
        match manifest.runtime.kind {
            RuntimeKind::Process => Supervisor::start_with_opts(
                record,
                self.host_methods.clone(),
                SupervisorOptions {
                    pidfile_dir: self.pidfile_dir.clone(),
                },
            )
            .map(Some)
            .map_err(SupervisorFactoryError::new),
            RuntimeKind::Builtin | RuntimeKind::Wasm => Ok(None),
        }
    }
}
