//! Debug & values DTOs — the live event model streamed from a running flow.
//!
//! When debug is enabled on a running flow, each pipeline node (the source, each
//! processor in order, and the sink) emits [`FlowDebugEvent`]s over SSE: periodic
//! per-node counters, sampled row values crossing the node boundary, and log
//! lines for retries / policy drops / DLQ / run errors. The UI fans these out by
//! `node_index` to overlay live values on both a table and the flow canvas.
//!
//! `node_index` is positional and stable for the current single-chain flow shape:
//! `0` is the source, `1..=N` are the processors in `pipeline` order, and `N+1`
//! is the sink. The optional `node_id` is reserved for a future branching shape;
//! consumers should prefer it when present and fall back to `node_index`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Which kind of node a debug event came from. Pairs with `node_index` so the UI
/// can label and colour the node without re-deriving its role from the position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    /// The pipeline input (node index 0).
    Source,
    /// A pipeline processor (node index 1..=N, in `pipeline` order).
    Processor,
    /// The pipeline output (node index N+1).
    Sink,
}

/// A periodic snapshot of one node's throughput since the run started. Rows in
/// and out differ for a processor that filters or fans out; for the source `in`
/// equals `out`, and for the sink `out` is what actually reached the writer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NodeCounters {
    /// Positional node index: 0 = source, 1..=N = processors, N+1 = sink.
    pub node_index: u32,
    /// The node's role.
    pub role: NodeRole,
    /// Rows that entered the node since the run started.
    pub rows_in: u64,
    /// Rows the node produced since the run started.
    pub rows_out: u64,
    /// Batches the node has handled since the run started.
    pub batches: u64,
}

/// Severity of a [`FlowDebugEvent::Log`] line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

/// One event in a running flow's debug stream. `kind`-tagged so the UI can switch
/// on the variant; every variant carries a monotonic `seq` for `Last-Event-ID`
/// resume parity with the transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FlowDebugEvent {
    /// A periodic per-node counter tick.
    Counters {
        /// Monotonic sequence number, assigned by the producer.
        seq: u64,
        #[serde(flatten)]
        counters: NodeCounters,
    },
    /// A bounded sample of rows that crossed a node boundary, as JSON objects.
    Sample {
        seq: u64,
        node_index: u32,
        role: NodeRole,
        /// Reserved for a future branching shape; prefer over `node_index` when
        /// present.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        node_id: Option<String>,
        /// Up to the per-node sample cap; each row is a JSON object.
        rows: Vec<Value>,
    },
    /// A log line — a retry attempt, a policy drop/DLQ, or the run-ending error.
    Log {
        seq: u64,
        level: LogLevel,
        /// The node the log relates to, when known.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        node_index: Option<u32>,
        message: String,
        /// Wall-clock millis since the epoch when the line was emitted.
        at_ms: u64,
    },
}

/// The state returned by the enable/disable endpoints so the UI can reflect
/// whether debug is currently capturing and how many nodes to expect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FlowDebugStatus {
    /// Whether value/sample capture is currently on for the running flow.
    pub enabled: bool,
    /// Total node count (source + processors + sink) so the UI can validate its
    /// positional mapping against the backend chain.
    pub node_count: u32,
}

/// The response to enabling debug on a running flow: the resulting status plus a
/// short-lived signed token and the SSE URL to open. A browser `EventSource`
/// cannot send an `Authorization` header, so the stream is authed by this token
/// in the query string — minted only after the Bearer-authed enable call passed
/// the flow's edit grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FlowDebugEnableResponse {
    #[serde(flatten)]
    pub status: FlowDebugStatus,
    /// The SSE endpoint to open, with the token already in the query string.
    pub stream_url: String,
    /// The signed stream token, also returned separately for clients that build
    /// their own URL.
    pub token: String,
    /// Seconds until the token expires.
    pub expires_in_secs: u64,
}
