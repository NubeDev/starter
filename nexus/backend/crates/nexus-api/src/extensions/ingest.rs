//! `ingest.*` host methods — the data-plane bridge a `process`/`wasm` extension
//! uses to feed a flow source (`ingest.write`) or drain a flow sink
//! (`ingest.read_batch`) without linking the engine's node traits.
//!
//! ## Tenancy
//!
//! The host stamps the **caller's** tenant onto every written row, taken from
//! the inbound `_meta.caller` identity (the supervisor binds it from the
//! extension's install). A `tenant_id` present in a pushed row is overwritten,
//! never trusted — so an extension can never widen a write past its caller's
//! tenant. A caller with no tenant is a hard deny (mirrors every other
//! tenant-scoped host method).
//!
//! ## Backpressure
//!
//! `ingest.write` enqueues into the flow's bounded channel via the same
//! [`IngestChannels`] seam the HTTP push path uses. A full channel returns
//! `retry_after_secs` and enqueues nothing (all-or-nothing per call) so a fast
//! extension throttles rather than the host buffering unboundedly.

use nexus_engine::{IngestChannels, IngestError};
use serde_json::Value;
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::ingest::{IngestWriteRequest, IngestWriteResponse};
use starter_ext_spi::warehouse::Row;
use starter_ext_spi::{Error as ExtError, Result as ExtResult};

/// The tenant column the host stamps onto every ingested row. Downstream
/// tenant-scoped sinks (datasource writers) read it; an extension cannot set it.
const TENANT_COLUMN: &str = "tenant_id";

/// Run `ingest.write`: stamp the caller's tenant onto each row and push the
/// batch into the named flow's bounded source channel.
///
/// `channels` is the flow manager's push registry; `source` is resolved against
/// it by name (the contributed source name is the flow id the host wired). A
/// full channel yields a back-pressure response with `retry_after_secs` and
/// enqueues nothing; an unknown/stopped flow is an error.
pub fn write(
    channels: &IngestChannels,
    params: Value,
    caller: Option<&CallerIdentity>,
) -> ExtResult<Value> {
    let req: IngestWriteRequest = serde_json::from_value(params)
        .map_err(|e| ExtError::extension_internal(format!("ingest.write params: {e}")))?;
    let tenant = caller_tenant(caller)?;

    // An empty push is a no-op accepted write — never touch the channel.
    if req.rows.is_empty() {
        return response(IngestWriteResponse {
            accepted: 0,
            retry_after_secs: None,
        });
    }

    let docs = stamp_and_encode(&req.rows, &tenant)?;
    let count = docs.len();
    match channels.try_push(&req.source, docs) {
        Ok(()) => response(IngestWriteResponse {
            accepted: count,
            retry_after_secs: None,
        }),
        Err(IngestError::Full { retry_after_secs }) => response(IngestWriteResponse {
            accepted: 0,
            retry_after_secs: Some(retry_after_secs),
        }),
        Err(IngestError::NotRunning) => Err(ExtError::extension_internal(format!(
            "ingest.write: source `{}` is not an accepting flow (not running)",
            req.source
        ))),
    }
}

/// Resolve the caller's tenant, refusing a caller with none — a write with no
/// tenant clamp would land rows with no owner, so it is a hard deny.
fn caller_tenant(caller: Option<&CallerIdentity>) -> ExtResult<String> {
    caller
        .and_then(|c| c.tenant_id.clone())
        .ok_or_else(|| ExtError::extension_internal("ingest.write requires a tenant-scoped caller"))
}

/// Stamp the tenant onto each row (overwriting any caller-supplied `tenant_id`)
/// and encode each as a JSON document string for the carrier batch.
fn stamp_and_encode(rows: &[Row], tenant: &str) -> ExtResult<Vec<String>> {
    rows.iter()
        .map(|row| {
            let mut map = row.as_map().clone();
            map.insert(TENANT_COLUMN.to_string(), Value::String(tenant.to_string()));
            serde_json::to_string(&map)
                .map_err(|e| ExtError::extension_internal(format!("ingest.write encode row: {e}")))
        })
        .collect()
}

/// Serialise an [`IngestWriteResponse`] into the host-method JSON return shape.
fn response(resp: IngestWriteResponse) -> ExtResult<Value> {
    serde_json::to_value(resp)
        .map_err(|e| ExtError::extension_internal(format!("ingest.write response: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::ingest::IngestWriteResponse;

    fn caller(tenant: Option<&str>) -> CallerIdentity {
        CallerIdentity {
            tenant_id: tenant.map(str::to_string),
            ..Default::default()
        }
    }

    fn row(json: serde_json::Value) -> Row {
        match json {
            serde_json::Value::Object(m) => Row::from_map(m),
            _ => panic!("row must be an object"),
        }
    }

    #[test]
    fn tenant_is_stamped_from_caller_not_payload() {
        // A row that lies about its tenant is overwritten with the caller's.
        let rows = vec![row(serde_json::json!({ "v": 1, "tenant_id": "evil" }))];
        let docs = stamp_and_encode(&rows, "t-real").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&docs[0]).unwrap();
        assert_eq!(parsed["tenant_id"], "t-real");
        assert_eq!(parsed["v"], 1);
    }

    #[test]
    fn missing_tenant_is_denied() {
        let err = write(
            &IngestChannels::new(),
            serde_json::json!({ "source": "f", "rows": [{ "v": 1 }] }),
            Some(&caller(None)),
        )
        .unwrap_err();
        assert!(err.to_string().contains("tenant"));
    }

    #[test]
    fn empty_push_is_a_noop_accepted_write() {
        let resp: IngestWriteResponse = serde_json::from_value(
            write(
                &IngestChannels::new(),
                serde_json::json!({ "source": "f", "rows": [] }),
                Some(&caller(Some("t-1"))),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.accepted, 0);
        assert!(resp.retry_after_secs.is_none());
    }

    #[test]
    fn unknown_source_is_not_running() {
        let err = write(
            &IngestChannels::new(),
            serde_json::json!({ "source": "nope", "rows": [{ "v": 1 }] }),
            Some(&caller(Some("t-1"))),
        )
        .unwrap_err();
        assert!(err.to_string().contains("not running"));
    }

    /// A burst of `ingest.write` calls into a one-deep push flow overruns the
    /// channel and surfaces the documented `retry_after_secs` back-pressure,
    /// enqueuing nothing on the full call. Mirrors the engine's deterministic
    /// 1-deep burst technique: synchronous pushes give the consumer task no
    /// chance to drain between calls.
    #[tokio::test]
    async fn full_channel_returns_retry_after() {
        use nexus_engine::FlowManager;
        use serde_json::json;

        let mgr = FlowManager::new().expect("register builders");
        let input = json!({ "type": "http_ingest", "capacity": 1 });
        let processors = vec![
            json!({ "type": "json_to_arrow" }),
            json!({ "type": "sql", "query": "SELECT v FROM flow" }),
        ];
        let output = json!({ "type": "drop" });
        mgr.start("ext-ingest-full", input, processors, output)
            .expect("start push flow");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let mut saw_retry = false;
        for n in 0..512 {
            let resp: IngestWriteResponse = serde_json::from_value(
                write(
                    mgr.ingest(),
                    json!({ "source": "ext-ingest-full", "rows": [{ "v": n }] }),
                    Some(&caller(Some("t-1"))),
                )
                .unwrap(),
            )
            .unwrap();
            if let Some(secs) = resp.retry_after_secs {
                assert!(secs >= 1, "retry-after hint is positive");
                assert_eq!(resp.accepted, 0, "nothing enqueued on a full channel");
                saw_retry = true;
                break;
            }
        }
        assert!(
            saw_retry,
            "a tight burst over a 1-deep channel back-pressures"
        );
        mgr.stop("ext-ingest-full");
    }
}
