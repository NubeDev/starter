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

pub mod authz;
pub mod behavior;
pub mod capability;
pub mod dashboard;
pub mod error;
pub mod event_bus;
pub mod fs_ext;
pub mod http_out;
pub mod id;
pub mod identity;
pub mod jsonrpc;
pub mod lifecycle;
pub mod manifest;
pub mod secrets;
pub mod tracing_ext;
pub mod wall_clock;
pub mod warehouse;

pub use behavior::ExtensionBehavior;
pub use capability::{Authority, Capability, PathSpec};
pub use error::{Error, Result};
pub use event_bus::{EventBusMessage, EventBusPublishRequest, EventBusSubscribeRequest};
pub use id::ExtensionId;
pub use identity::{CallerIdentity, FrameMeta};
pub use jsonrpc::{
    flow_node_error_codes, stream_methods, JsonRpcEnvelope, JsonRpcId, JsonRpcNotification,
    JsonRpcRequest, JsonRpcResponse, JsonRpcResponsePayload, StreamCancel, StreamEnd, StreamError,
    StreamEvent, StreamId, StreamNotification, FLOW_NODE_INVOKE, JSONRPC_VERSION,
};
pub use lifecycle::LifecycleState;
pub use manifest::{
    AuthGate, Backoff, CliStreaming, ContributeAnomalyRule, ContributeCli, ContributeGrpc,
    ContributeNode, ContributeRest, ContributeSkillsDir, ContributeTool, ContributeUi,
    ContributeUiExpose, ContributeWarehouseTable, ContributeWarehouseTemplate, ContributeWorker,
    Contributes, HealthConfig, Manifest, ManifestRequires, OnErrorPolicy, PermissionGate, Require,
    RestStreaming, RestartPolicy, RetryStrategy, Runtime, RuntimeKind, Supervision, TableColumn,
    MANIFEST_VERSION,
};
pub use warehouse::{
    Row, TemplateSpec, WarehouseDeleteRequest, WarehouseDeleteResponse, WarehouseReadRequest,
    WarehouseReadResponse, WarehouseUpdateRequest, WarehouseUpdateResponse, WarehouseWriteRequest,
    WarehouseWriteResponse,
};
