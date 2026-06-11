//! Resolve a flow's `datasource` output node into the engine's resolved sink
//! config before the pipeline is built.
//!
//! A saved flow's output references a datasource by id
//! (`{"type":"datasource","datasource":"<uuid>","table":"…"}`). The engine sink
//! cannot read the control-plane database or decrypt a secret, so the id is
//! resolved here — through the audited [`nexus_store::datasource`] boundary — into
//! the connection material the engine consumes. A non-datasource output (a raw
//! `postgres` sink, `sse`, `drop`, …) passes through untouched, so legacy stored
//! configs keep working with no rewrite.

use nexus_store::datasource;
use serde_json::Value;
use starter_spi::Error;
use uuid::Uuid;

use crate::state::AppState;

/// Rewrite `output` if it is a `datasource`-typed sink: look up the referenced
/// datasource for `tenant`, decrypt its secret (audited as `actor`), and return
/// the engine's resolved `datasource` sink config. Any other output is returned
/// unchanged.
///
/// Errors propagate the store's `NotFound`/`Invalid` so the start handler maps
/// them to the right status — a flow naming a missing or unsupported datasource
/// must fail loudly, not start with a dead sink.
pub async fn resolve_flow_output(
    state: &AppState,
    tenant: &str,
    actor: &str,
    output: Value,
) -> Result<Value, Error> {
    if output.get("type").and_then(Value::as_str) != Some("datasource") {
        return Ok(output);
    }
    let id = output
        .get("datasource")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Invalid {
            message: "datasource output requires a string \"datasource\" id".into(),
        })?;
    let id = Uuid::parse_str(id).map_err(|_| Error::Invalid {
        message: format!("datasource output id {id:?} is not a uuid"),
    })?;
    let table = output
        .get("table")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Invalid {
            message: "datasource output requires a string \"table\"".into(),
        })?;
    let batch_rows = output.get("batch_rows").and_then(Value::as_u64);
    let batch_ms = output.get("batch_ms").and_then(Value::as_u64);

    datasource::resolve_sink_config(
        &state.metadata,
        &state.envelope,
        tenant,
        actor,
        id,
        table,
        batch_rows,
        batch_ms,
    )
    .await
}
