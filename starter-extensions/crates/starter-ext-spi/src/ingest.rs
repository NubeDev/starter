//! Ingest data-plane — shared wire types for the `ingest.*` host methods.
//!
//! These let a `process`/`wasm` extension feed data **into** a running flow's
//! bounded source channel (`ingest.write`) and drain batches a flow's sink
//! produced (`ingest.read_batch`), without linking the engine's node traits.
//! The bridge is host-mediated: the extension declares a source/sink in its
//! manifest (`contributes.sources[]` / `contributes.sinks[]`), the host wires
//! it to the named flow, and these requests carry the data across the JSON-RPC
//! boundary.
//!
//! Tenancy is **never** read from these payloads. The host stamps the caller's
//! tenant onto every written row from the inbound `_meta.caller` identity (the
//! supervisor binds it from the extension's install), so a payload `tenant_id`
//! can never widen the write. See
//! [`docs/design/extensions/caller-identity.md`](../../../../rubix/docs/design/extensions/caller-identity.md).
//!
//! Backpressure is explicit: `ingest.write` returns
//! [`IngestWriteResponse::retry_after_secs`] when the target flow's bounded
//! channel is full, so a fast extension throttles rather than the host buffering
//! unboundedly. This mirrors the HTTP push path's `429 + Retry-After`.

use serde::{Deserialize, Serialize};

use crate::warehouse::Row;

/// `ingest.write` request — push JSON rows into a named flow source.
///
/// `source` names the flow whose `http_ingest` source receives the rows (the
/// contributed source name the extension declared, resolved by the host to a
/// running flow). `rows` are opaque JSON objects; the host stamps the caller's
/// tenant onto each before they enter the channel — a `tenant_id` in a row is
/// ignored, never trusted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IngestWriteRequest {
    /// The contributed source / flow name to push into.
    pub source: String,
    /// The rows to ingest. An empty vec is a no-op accepted write.
    #[serde(default)]
    pub rows: Vec<Row>,
}

/// `ingest.write` response — how many rows were accepted, and the backpressure
/// hint when the channel was full.
///
/// When `retry_after_secs` is `Some`, **no** rows were enqueued (the push is
/// all-or-nothing per call): the caller must retry after the hinted delay. When
/// it is `None`, `accepted` equals the request's row count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IngestWriteResponse {
    /// Rows enqueued into the flow's channel. `0` when back-pressured.
    pub accepted: usize,
    /// Set when the channel was full: seconds to wait before retrying. The
    /// write did not happen; nothing was enqueued.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
}

/// `ingest.read_batch` request — long-poll for the next batch a flow sink
/// produced for this extension's contributed sink.
///
/// `sink` names the contributed sink to drain. `max_rows` caps the returned
/// batch; the host returns up to that many rows from the sink's bounded output
/// queue, or an empty batch if none are ready within the host's poll window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IngestReadBatchRequest {
    /// The contributed sink / flow name to drain.
    pub sink: String,
    /// Maximum rows to return in this batch. Omitted ⇒ host default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rows: Option<usize>,
}

/// `ingest.read_batch` response — the rows drained from the sink's output queue.
///
/// An empty `rows` vec means nothing was ready (the long-poll timed out); the
/// extension simply polls again. Rows are the JSON the sink emitted, tenant
/// already applied by the producing flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IngestReadBatchResponse {
    /// The drained rows. Empty when the poll window elapsed with no data.
    pub rows: Vec<Row>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_request_defaults_rows_to_empty() {
        let req: IngestWriteRequest = serde_json::from_value(serde_json::json!({
            "source": "com.acme.weather.in"
        }))
        .unwrap();
        assert_eq!(req.source, "com.acme.weather.in");
        assert!(req.rows.is_empty());
    }

    #[test]
    fn write_response_omits_retry_when_accepted() {
        let resp = IngestWriteResponse {
            accepted: 3,
            retry_after_secs: None,
        };
        let j = serde_json::to_value(&resp).unwrap();
        assert_eq!(j["accepted"], 3);
        assert!(j.get("retry_after_secs").is_none());
    }

    #[test]
    fn read_batch_request_round_trip() {
        let req = IngestReadBatchRequest {
            sink: "com.acme.weather.out".into(),
            max_rows: Some(128),
        };
        let back: IngestReadBatchRequest =
            serde_json::from_value(serde_json::to_value(&req).unwrap()).unwrap();
        assert_eq!(back, req);
    }
}
