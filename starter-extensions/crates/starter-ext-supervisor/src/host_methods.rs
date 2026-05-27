//! Host-method dispatch trait.
//!
//! When a process-flavour extension sends a JSON-RPC request whose
//! `method` is a capability-gated host method (e.g.
//! `dashboard.read`, `authz.check`, `warehouse.query`), the
//! supervisor first runs the request through [`crate::CapabilityGate`]
//! and then — if the gate allows — hands it to the
//! [`HostMethodHandler`] this crate's `Supervisor::start_with`
//! installer attached.
//!
//! v0.1 ships a [`NotImplementedHandler`] default that replies with
//! `Error::ExtensionInternal("host method <m> not implemented in
//! v0.1 supervisor")` — the behaviour the supervisor used to
//! emit inline before this trait existed. Hosts opt into real
//! plumbing by passing their own handler at `Supervisor::start_with`.
//!
//! ## Wire shape
//!
//! - `method` is the gated method string (e.g. `"dashboard.read"`).
//! - `params` is whatever the child sent verbatim. Handlers
//!   deserialise via the [`starter_ext_spi::dashboard`] /
//!   [`starter_ext_spi::authz`] wire types.
//! - `caller` is the `_meta.caller` sidecar from the inbound
//!   envelope (`None` for system / host-internal calls).
//!
//! Handlers return the JSON value the supervisor should put in
//! `result` on success, or an `Error` for the `error` slot. The
//! supervisor handles the framing, id correlation, and the
//! `_meta.caller` re-stamping back onto the response.

use async_trait::async_trait;
use std::sync::Arc;

use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::{Error, ExtensionId, Result};

/// Host-installable handler for inbound capability-gated requests
/// from a process-flavour child.
///
/// Each method invocation runs on the supervisor's async task; the
/// handler may itself dispatch into per-call rubix backends,
/// invoke the SDUI dashboard store, evaluate authz policies, etc.
/// The supervisor never spawns blocking work on this thread — if
/// the handler needs to bridge sync/async, it is the handler's
/// responsibility (the rubix-side `block_in_place`/`block_on`
/// pattern is what landed in earlier slices).
#[async_trait]
pub trait HostMethodHandler: Send + Sync + 'static {
    /// Resolve one host call.
    ///
    /// - `extension` is the calling extension's id (the supervisor
    ///   already verified gate membership for this id).
    /// - `method` is the gated method name (verbatim; the gate
    ///   already verified it is admissible).
    /// - `params` is the JSON params from the inbound envelope.
    /// - `caller` is the `_meta.caller` sidecar, if any.
    async fn call(
        &self,
        extension: &ExtensionId,
        method: &str,
        params: serde_json::Value,
        caller: Option<&CallerIdentity>,
    ) -> Result<serde_json::Value>;
}

/// Default handler that replies `Error::ExtensionInternal("host
/// method <m> not implemented in v0.1 supervisor")` to every
/// invocation. Matches the supervisor's pre-trait behaviour so
/// `Supervisor::start(record)` is byte-for-byte unchanged when a
/// host doesn't opt in.
#[derive(Debug, Default, Clone, Copy)]
pub struct NotImplementedHandler;

#[async_trait]
impl HostMethodHandler for NotImplementedHandler {
    async fn call(
        &self,
        _extension: &ExtensionId,
        method: &str,
        _params: serde_json::Value,
        _caller: Option<&CallerIdentity>,
    ) -> Result<serde_json::Value> {
        Err(Error::extension_internal(format!(
            "host method {method:?} not implemented in v0.1 supervisor"
        )))
    }
}

/// Convenience alias for the shared-Arc shape consumers thread
/// through `Supervisor::start_with`.
pub type SharedHostMethodHandler = Arc<dyn HostMethodHandler>;
