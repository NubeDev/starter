//! # starter-ext-spi
//!
//! Contracts crate for the `starter-extensions` workspace. The wire types,
//! trait seams, and manifest schema that every other crate in the workspace
//! (host, sdk, supervisor, wasm, server, adapters) shares.
//!
//! Per `DOCS/extensions/scope/SCOPE.md` rule **R2**:
//!
//! - This crate depends only on `starter-spi` from the parent workspace.
//! - Every other crate in `starter-extensions` depends on this one.
//! - Zero runtime logic, zero I/O, zero process spawning, zero HTTP.
//!
//! The body of every public item lives in its own file under
//! `src/<concept>.rs`. This file is a re-export barrel — keep it that way.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod behavior;
pub mod capability;
pub mod error;
pub mod id;
pub mod jsonrpc;
pub mod lifecycle;
pub mod manifest;

pub use behavior::ExtensionBehavior;
pub use capability::{Authority, Capability, PathSpec};
pub use error::{Error, Result};
pub use id::ExtensionId;
pub use jsonrpc::{
    stream_methods, JsonRpcEnvelope, JsonRpcId, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, JsonRpcResponsePayload, StreamEnd, StreamError, StreamEvent, StreamId,
    StreamNotification, JSONRPC_VERSION,
};
pub use lifecycle::LifecycleState;
pub use manifest::{
    AuthGate, Backoff, CliStreaming, ContributeCli, ContributeGrpc, ContributeNode, ContributeRest,
    ContributeSkillsDir, ContributeTool, ContributeUi, ContributeUiExpose, ContributeWorker,
    Contributes, HealthConfig, Manifest, ManifestRequires, OnErrorPolicy, Require, RestStreaming,
    RestartPolicy, RetryStrategy, Runtime, RuntimeKind, Supervision, MANIFEST_VERSION,
};
