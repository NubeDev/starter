//! Cross-instance `flows_definitions` reload listener.
//!
//! The Phase D.1 trigger on `flows_definitions` fires
//! `NOTIFY rubix_flows_definitions` on every insert/update.
//! This module owns the consumer side: one [`PgListener`] per
//! rubix-agent process, listening on the
//! [`FLOWS_DEFINITIONS_CHANNEL`] channel, fetching the freshly
//! inserted/updated row's `body_yaml`, and calling the
//! [`ReloadFn`] hook with the parsed `(FlowId, FlowRevisionId,
//! FlowBody)` triple so the in-process `FlowRegistry` can pick
//! up the new revision without a redeploy.
//!
//! Why a hook rather than a direct `FlowRegistry::reload` call:
//! `FlowRegistry` is parameterised by `NodeKindRegistry` at
//! register time and the registration shape (seed adapter, output
//! adapter, terminal slots) is the responsibility of
//! `boot::mcp::register`. Passing a closure keeps this module
//! free of those concerns and matches the verb-per-file rule.
//!
//! ## Payload contract
//!
//! The trigger emits JSON of the form
//! `{ "op": "INSERT"|"UPDATE", "id": "...", "tenant_id": "...",
//!    "flow_id": "...", "revision_id": "...", "superseded_at":
//!    null|"..." }`. The listener filters to non-superseded
//! events and re-reads the row from the table to avoid trusting
//! payload-side body text (payloads have an 8000-byte limit).
//!
//! ## Lifecycle
//!
//! [`spawn_flow_notify`] returns a [`JoinHandle`]; dropping the
//! handle (or aborting the runtime) shuts the listener down.
//! When `dsn` is `None` this returns `Ok(None)` so a laptop boot
//! without Postgres stays quiet.

use std::sync::Arc;

use anyhow::Result;
use serde::Deserialize;
use sqlx::postgres::PgListener;
use starter_flow::definition::body::FlowBody;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_store_postgres::pool::{connect, Pool};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};
use uuid::Uuid;

use rubix_store_postgres::FLOWS_DEFINITIONS_CHANNEL;

/// Callback the listener invokes for every live revision it
/// reads. Returns a future so the registry reload can be async
/// (mirrors `FlowRegistry::register`'s signature).
pub type ReloadFn = Arc<
    dyn Fn((FlowId, FlowRevisionId, FlowBody)) -> futures::future::BoxFuture<'static, Result<()>>
        + Send
        + Sync,
>;

#[derive(Debug, Deserialize)]
struct NotifyPayload {
    op: String,
    flow_id: String,
    revision_id: String,
    #[serde(default)]
    superseded_at: Option<String>,
    #[serde(default)]
    tenant_id: Option<Uuid>,
}

/// Spawn the LISTEN loop. Returns `Ok(None)` when `dsn` is
/// `None`. The returned task lives until aborted; drop the
/// handle to stop the listener.
pub async fn spawn_flow_notify(
    dsn: Option<&str>,
    on_reload: ReloadFn,
) -> Result<Option<JoinHandle<()>>> {
    let Some(dsn) = dsn else {
        warn!(
            target: "rubix.boot",
            "Postgres DSN unset — skipping flows_definitions NOTIFY listener",
        );
        return Ok(None);
    };
    let pool = connect(dsn)
        .await
        .map_err(|e| anyhow::anyhow!("connect for flow_notify: {e}"))?;
    let mut listener = PgListener::connect_with(pool.sqlx())
        .await
        .map_err(|e| anyhow::anyhow!("PgListener::connect_with: {e}"))?;
    listener
        .listen(FLOWS_DEFINITIONS_CHANNEL)
        .await
        .map_err(|e| anyhow::anyhow!("LISTEN {FLOWS_DEFINITIONS_CHANNEL}: {e}"))?;
    info!(
        channel = FLOWS_DEFINITIONS_CHANNEL,
        "flows_definitions NOTIFY listener active"
    );

    let handle = tokio::spawn(async move {
        loop {
            let notification = match listener.recv().await {
                Ok(n) => n,
                Err(e) => {
                    warn!(error = %e, "flow_notify recv failed; sleeping before retry");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            };
            if let Err(e) = handle_payload(&pool, notification.payload(), &on_reload).await {
                warn!(error = %e, payload = notification.payload(), "flow_notify dispatch failed");
            }
        }
    });
    Ok(Some(handle))
}

async fn handle_payload(pool: &Pool, payload: &str, on_reload: &ReloadFn) -> Result<()> {
    let parsed: NotifyPayload = serde_json::from_str(payload)
        .map_err(|e| anyhow::anyhow!("payload not json: {e}"))?;
    if parsed.superseded_at.is_some() {
        debug!(flow_id = %parsed.flow_id, "flow_notify: superseded — skipping reload");
        return Ok(());
    }
    debug!(
        op = parsed.op,
        flow_id = %parsed.flow_id,
        revision_id = %parsed.revision_id,
        "flow_notify: fetching body for reload"
    );

    // Re-read the body from PG rather than trusting payload text
    // (NOTIFY payload is capped at 8000 bytes; the full YAML can
    // exceed that). Filtering by tenant when present keeps us
    // honest under future multi-tenant routing.
    let body_yaml: Option<String> = match parsed.tenant_id {
        Some(tenant) => sqlx::query_scalar(
            "SELECT body_yaml FROM flows_definitions
              WHERE tenant_id = $1::uuid
                AND flow_id   = $2
                AND revision_id = $3",
        )
        .bind(tenant)
        .bind(&parsed.flow_id)
        .bind(&parsed.revision_id)
        .fetch_optional(pool.sqlx())
        .await
        .map_err(|e| anyhow::anyhow!("select body_yaml: {e}"))?,
        None => sqlx::query_scalar(
            "SELECT body_yaml FROM flows_definitions
              WHERE flow_id = $1 AND revision_id = $2",
        )
        .bind(&parsed.flow_id)
        .bind(&parsed.revision_id)
        .fetch_optional(pool.sqlx())
        .await
        .map_err(|e| anyhow::anyhow!("select body_yaml: {e}"))?,
    };
    let Some(body_yaml) = body_yaml else {
        warn!(flow_id = %parsed.flow_id, revision = %parsed.revision_id,
            "flow_notify: row vanished before reload (deleted?)");
        return Ok(());
    };

    let path = format!("pg://flows_definitions/{}", parsed.flow_id);
    let yaml = rubix_flows::parse_yaml(&path, body_yaml.as_bytes())
        .map_err(|e| anyhow::anyhow!("parse pg yaml: {e}"))?;
    let (flow_id, _rev, body) = rubix_flows::convert(&path, yaml)
        .map_err(|e| anyhow::anyhow!("convert pg yaml: {e}"))?;
    let revision = parsed
        .revision_id
        .parse::<Uuid>()
        .map(FlowRevisionId)
        .map_err(|e| anyhow::anyhow!("revision_id not uuid: {e}"))?;

    (on_reload)((flow_id, revision, body)).await
}
