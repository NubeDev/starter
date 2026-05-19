//! Flow-level contracts: ids, the run event stream, store seams.
//!
//! Per `DOCS/flow/scope/SCOPE.md` R6 (sessions persist; runs persist;
//! checkpoints are engine-typed) and R8 (flows surface as Tools and as
//! Services). The `FlowStore` / `RunStore` traits are deliberately
//! empty seams in Phase 1 — CRUD method shapes are documented but not
//! required yet; Phase 3 fleshes them out alongside the
//! `starter-store-sqlite` implementations.

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::node::{IdError, NodeError, NodeId, SlotMap, SlotValue};

/// Reverse-DNS flow identifier (SCOPE R10). Same validation rules as
/// [`NodeId`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FlowId(String);

impl FlowId {
    /// Parse a string as a flow id.
    pub fn new(s: impl Into<String>) -> Result<Self, IdError> {
        let s = s.into();
        crate::node::validate_reverse_dns(&s)?;
        Ok(Self(s))
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FlowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for FlowId {
    type Error = IdError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<FlowId> for String {
    fn from(value: FlowId) -> Self {
        value.0
    }
}

/// UUID-backed identifier for a specific revision of a flow.
///
/// SCOPE "Decisions made": "revisions are immutable; `head_seq` pointer
/// per flow tracks the current revision". The revision id is the
/// immutable handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FlowRevisionId(pub Uuid);

impl FlowRevisionId {
    /// Generate a fresh revision id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FlowRevisionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for FlowRevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// UUID-backed identifier for a single flow run (one invocation, start
/// to terminal state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(pub Uuid);

impl RunId {
    /// Generate a fresh run id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Events streamed for the lifetime of a flow run.
///
/// SCOPE R13: `Stream<FlowEvent>` — same shape `starter-ai`'s `OnEvent`
/// and `starter_spi`'s event channels already use. Adapters render
/// natively per transport (SSE, NDJSON, MCP `notifications/progress`,
/// gRPC server-streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlowEvent {
    /// Run started. Emitted once at the top of every run.
    RunStarted {
        /// The run id.
        run: RunId,
        /// The flow being run.
        flow: FlowId,
    },
    /// A node started executing.
    NodeStarted {
        /// The run id.
        run: RunId,
        /// The node that started.
        node: NodeId,
    },
    /// A node emitted a value on one of its output slots.
    NodeEmitted {
        /// The run id.
        run: RunId,
        /// The node that emitted.
        node: NodeId,
        /// The output slot name on that node.
        slot: String,
        /// The value written.
        value: SlotValue,
    },
    /// A node failed.
    NodeFailed {
        /// The run id.
        run: RunId,
        /// The node that failed.
        node: NodeId,
        /// String form of the underlying [`NodeError`]. Kept as a
        /// string here so [`FlowEvent`] stays `Serialize` regardless
        /// of the concrete error variant a kind returns.
        error: String,
    },
    /// Run completed normally. Output is the terminal-node output
    /// map (per R9, this is the value `FlowAsTool` returns).
    RunCompleted {
        /// The run id.
        run: RunId,
        /// The terminal output map.
        output: SlotMap,
    },
    /// Run failed.
    RunFailed {
        /// The run id.
        run: RunId,
        /// String form of the underlying [`FlowError`].
        error: String,
    },
    /// Run was cancelled via its [`Cancel`](crate::Cancel) token.
    RunCancelled {
        /// The run id.
        run: RunId,
    },
}

impl FlowEvent {
    /// Convenience constructor for [`FlowEvent::NodeFailed`].
    pub fn node_failed(run: RunId, node: NodeId, err: &NodeError) -> Self {
        Self::NodeFailed {
            run,
            node,
            error: err.to_string(),
        }
    }

    /// Convenience constructor for [`FlowEvent::RunFailed`].
    pub fn run_failed(run: RunId, err: &FlowError) -> Self {
        Self::RunFailed {
            run,
            error: err.to_string(),
        }
    }
}

/// Errors that fail an entire run.
///
/// Phase 1 ships the shape; concrete typed variants land alongside the
/// engine in Phase 2 (e.g. `CycleBudgetExhausted` per R1).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FlowError {
    /// Per-run cycle budget exhausted (R1).
    #[error("cycle budget exhausted after {hops} propagation hops")]
    CycleBudgetExhausted {
        /// Hop count at which the cap fired.
        hops: u64,
    },
    /// A node returned a [`NodeError`] under an `on_failure: abort`
    /// policy (R3).
    #[error("node {node} failed: {error}")]
    NodeAborted {
        /// The aborting node.
        node: NodeId,
        /// The underlying node error, stringified for portability.
        error: String,
    },
    /// Run-store / flow-store backend failure.
    #[error("flow backend failure: {0}")]
    Backend(String),
}

/// Persistence seam for flow definitions (R6).
///
/// Phase 1 is intentionally an empty trait — the CRUD method shape
/// (`load`, `put`, `list`, `revisions`, `head`) is documented in
/// SCOPE's "Decisions made" block but is not required until Phase 3
/// implements it in `starter-store-sqlite`. Locking the shape in
/// Phase 1 would force premature decisions on revision history,
/// listing pagination, and tenancy boundaries.
#[async_trait]
pub trait FlowStore: Send + Sync + 'static {}

/// Persistence seam for flow runs and per-run checkpoints (R6).
///
/// Same Phase 1 posture as [`FlowStore`]: the CRUD + checkpoint method
/// shape is reserved for Phase 3. R6 fixes the *types* — checkpoints
/// are engine-typed `RunState` plus per-node opaque blobs — and that
/// fix is what lets the trait stay empty here without forcing future
/// breakage.
#[async_trait]
pub trait RunStore: Send + Sync + 'static {}
