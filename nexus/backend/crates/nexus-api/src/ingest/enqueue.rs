//! Enqueue a pushed JSON body onto a flow's bounded ingest channel.

use nexus_engine::IngestError;
use nexus_store::flow;
use serde_json::Value;
use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use crate::state::AppState;

/// Why a push could not be accepted, in domain terms the route maps to HTTP.
#[derive(Debug)]
pub enum EnqueueError {
    /// No such flow in the caller's tenant, or it is not accepting pushes (not a
    /// running `http_ingest` flow). The route answers `404` — a cross-tenant probe
    /// is indistinguishable from a missing flow, so it never leaks existence.
    NotFound,
    /// The flow's channel is full. The route answers `429 + Retry-After`. Carries
    /// the suggested back-off in seconds.
    Full { retry_after_secs: u64 },
    /// A store lookup failed.
    Store(Error),
}

/// Enqueue `body`'s documents onto `flow_id`'s ingest channel for `tenant`,
/// returning how many documents were accepted.
///
/// The body is a JSON object (one document) or an array of objects (one document
/// each); both converge on the flow's `json_to_arrow` shaping. The flow is first
/// resolved within `tenant` so a push to another tenant's flow is a `NotFound`,
/// then the documents are tried onto the bounded channel without blocking — a full
/// channel is the backpressure signal, surfaced as [`EnqueueError::Full`].
pub async fn enqueue(
    state: &AppState,
    tenant: &str,
    flow_id: Uuid,
    body: Value,
) -> Result<u64, EnqueueError> {
    require_tenant_flow(&state.metadata, tenant, flow_id).await?;
    let docs = documents(body);
    let count = docs.len() as u64;
    state
        .flows
        .ingest()
        .try_push(&flow_id.to_string(), docs)
        .map_err(|e| match e {
            IngestError::NotRunning => EnqueueError::NotFound,
            IngestError::Full { retry_after_secs } => EnqueueError::Full { retry_after_secs },
        })?;
    Ok(count)
}

/// Resolve the flow within the caller's tenant, mapping an absent row to
/// `NotFound` so a cross-tenant push cannot tell a foreign flow from a missing one.
async fn require_tenant_flow(
    metadata: &PgPool,
    tenant: &str,
    flow_id: Uuid,
) -> Result<(), EnqueueError> {
    match flow::get(metadata, tenant, flow_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(EnqueueError::NotFound),
        Err(e) => Err(EnqueueError::Store(e)),
    }
}

/// Flatten a pushed body into JSON-document strings: an array yields one document
/// per element, any other value yields a single document — the same convention the
/// `http_poll` source uses so a scalar and an array push both flow uniformly.
fn documents(body: Value) -> Vec<String> {
    match body {
        Value::Array(items) => items.into_iter().map(|v| v.to_string()).collect(),
        other => vec![other.to_string()],
    }
}
