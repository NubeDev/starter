//! `GET /api/v1/flows/{flow_id}/events` — SSE live tail of every
//! `FlowEvent` the engine fans out for `flow_id`.
//!
//! The route owns no buffering of its own — it subscribes to the
//! per-flow `tokio::sync::broadcast::Sender` held in the shared
//! [`FlowSubscriptionRegistry`] (see [`crate::boot::flow_runtime`]).
//! Every connect calls
//! [`FlowSubscriptionRegistry::subscribe_or_create`], which lazily
//! installs a sender on first connect; the engine-side run pump
//! reuses that sender via [`FlowSubscriptionRegistry::sender`].
//!
//! Wire shape:
//!
//!   - One SSE `data:` frame per [`FlowEvent::NodeEmitted`], JSON
//!     body matching [`NodeSlotValue`]. All other event variants are
//!     swallowed today (the SSE channel is the "slot values for the
//!     UI live view" channel, not a debug firehose). When richer
//!     variants need surfacing they land as named SSE `event:` types
//!     so the existing `data:`-only subscribers stay compatible.
//!   - 15 s keep-alive heartbeats (matches the extensions-events
//!     route at `starter-extensions/crates/starter-ext-server/src/events.rs`).
//!   - Broadcast-`Lagged` errors close the stream — the SSE client
//!     reconnects via `EventSource`'s built-in retry and re-reads
//!     the current state from `/flows/{flow_id}` if it needs a
//!     fresh snapshot.
//!
//! CSRF gating: SSE responses follow the existing extension-events
//! pattern — the route is mounted *outside* the CSRF middleware
//! sandwich because `text/event-stream` GETs cannot carry a request
//! body and the browser-side `EventSource` cannot forward a CSRF
//! token header. Authentication still gates the route via the
//! standard `with_principal` layer applied by `main.rs` when a
//! database is configured; without a DSN the route stays open on
//! the laptop dev path, mirroring the tools router fallback.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures::stream::Stream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tracing::debug;

use starter_flow_spi::event_dto::NodeSlotValue;
use starter_flow_spi::flow::{FlowEvent, FlowId};

use crate::boot::flow_runtime::FlowSubscriptionRegistry;

/// State threaded into the SSE handler.
#[derive(Clone)]
pub struct FlowEventsState {
    pub subscriptions: Arc<FlowSubscriptionRegistry>,
}

/// Build the router. Mount under `/api/v1`.
pub fn router(state: FlowEventsState) -> Router {
    Router::new()
        .route("/api/v1/flows/{flow_id}/events", get(events))
        .with_state(state)
}

async fn events(
    State(state): State<FlowEventsState>,
    Path(flow_id_raw): Path<String>,
) -> axum::response::Response {
    let flow_id = match FlowId::new(&flow_id_raw) {
        Ok(id) => id,
        Err(e) => {
            debug!(target: "rubix.routes.flow_events", flow_id = %flow_id_raw, error = %e, "rejecting non-reverse-dns flow id");
            return (StatusCode::BAD_REQUEST, format!("invalid flow_id: {e}")).into_response();
        }
    };
    let rx = state.subscriptions.subscribe_or_create(&flow_id).await;
    let stream = to_sse_stream(rx);
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

/// Map a per-flow `broadcast::Receiver<FlowEvent>` into the SSE
/// item stream the axum responder consumes. Each `NodeEmitted` is
/// projected through [`NodeSlotValue::from_event`] and serialised as
/// the SSE `data:` body; every other variant is filtered out so the
/// live-tail wire shape stays focused on slot values.
fn to_sse_stream(
    rx: broadcast::Receiver<FlowEvent>,
) -> impl Stream<Item = Result<SseEvent, Infallible>> {
    BroadcastStream::new(rx).filter_map(|item| match item {
        Ok(ev) => NodeSlotValue::from_event(&ev).map(|nsv| {
            let json = serde_json::to_string(&nsv).unwrap_or_else(|_| "{}".to_string());
            Ok(SseEvent::default().data(json))
        }),
        Err(_lagged) => {
            // Stream close on lag — `EventSource` reconnects.
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_flow_spi::flow::RunId;
    use starter_flow_spi::node::{NodeId, SlotValue};

    #[tokio::test]
    async fn stream_projects_node_emitted_and_drops_other_variants() {
        let reg = Arc::new(FlowSubscriptionRegistry::new());
        let flow = FlowId::new("dev.starter.echo").unwrap();
        let rx = reg.subscribe_or_create(&flow).await;
        let tx = reg.sender(&flow).await;
        let mut stream = Box::pin(to_sse_stream(rx));
        let run = RunId::new();
        tx.send(FlowEvent::RunStarted {
            run,
            flow: flow.clone(),
        })
        .unwrap();
        tx.send(FlowEvent::NodeEmitted {
            run,
            node: NodeId::new("dev.starter.counter").unwrap(),
            slot: "count".into(),
            value: SlotValue::Int(42),
        })
        .unwrap();
        // Drop tx so the stream closes after we drain it.
        drop(tx);
        drop(reg);
        let first = futures::StreamExt::next(&mut stream)
            .await
            .expect("one event")
            .unwrap();
        let data = format!("{first:?}");
        assert!(
            data.contains("\\\"count\\\""),
            "unexpected sse frame: {data}"
        );
        assert!(data.contains("42"));
    }
}
