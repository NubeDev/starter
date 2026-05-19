//! `GET /extensions/<id>/events` — paginated snapshot + SSE live tail.
//!
//! Two modes, distinguished at request time:
//!
//! 1. **Snapshot** (default). Returns `{ events: [...], next_seq: u64 }`
//!    where `next_seq` is the cursor a client passes back as `?after=`
//!    to fetch only newer entries. Supports `?after=<seq>` and
//!    `?limit=<n>` (limit caps at 1000, the ring capacity).
//!
//! 2. **Live tail (SSE)**. Triggered by `Accept: text/event-stream` or
//!    `?stream=1`. Emits the current snapshot followed by new events
//!    as the supervisor pushes them into the ring. Polling cadence is
//!    250 ms — small enough to feel live, infrequent enough that an
//!    idle admin tab costs ~0 CPU.
//!
//! The polling-based live tail is intentional v0.1 simplicity: the
//! supervisor doesn't yet expose a broadcast channel, and the ring is
//! cheap to lock. A v0.2 upgrade can add `tokio::sync::broadcast` to
//! the supervisor without changing the wire shape.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use starter_ext_spi::ExtensionId;
use starter_ext_supervisor::{RingEvent, SupervisorHandle};

use crate::admin::ExtensionAdmin;

const DEFAULT_LIMIT: usize = 1000;
const POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Deserialize)]
pub(crate) struct EventsQuery {
    /// Only return events with `seq > after`. Defaults to 0 (all).
    #[serde(default)]
    pub after: u64,
    /// Cap the number of events returned. Defaults to the ring size.
    #[serde(default)]
    pub limit: Option<usize>,
    /// Force the SSE upgrade without setting the `Accept` header.
    /// Useful for `curl` / `EventSource` polyfills.
    #[serde(default)]
    pub stream: Option<u8>,
}

#[derive(Debug, Serialize)]
pub(crate) struct EventsPage {
    pub events: Vec<RingEvent>,
    /// `seq` value the *next* event will receive. Pass back as `?after=`
    /// to resume the stream without overlap.
    pub next_seq: u64,
}

pub(crate) async fn events(
    State(admin): State<ExtensionAdmin>,
    Path(id): Path<String>,
    Query(q): Query<EventsQuery>,
    headers: HeaderMap,
) -> axum::response::Response {
    let parsed_id = match ExtensionId::new(&id) {
        Ok(id) => id,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    // Resolve the supervisor handle. Builtin / disabled records have
    // no handle — surface that as 404 so the admin UI can stop polling
    // (rather than 204 / empty 200 that hides the absence).
    let handle = match admin.supervisor(&parsed_id) {
        Some(h) => h,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    if wants_sse(&headers, q.stream) {
        let stream = sse_stream(handle, q.after);
        return Sse::new(stream)
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
            .into_response();
    }

    // Snapshot mode.
    let snapshot = handle.events();
    let mut filtered: Vec<RingEvent> = snapshot
        .into_iter()
        .filter(|e| e.seq > q.after)
        .collect();
    if let Some(limit) = q.limit {
        let limit = limit.min(DEFAULT_LIMIT);
        if filtered.len() > limit {
            filtered.truncate(limit);
        }
    }
    let next_seq = filtered
        .last()
        .map(|e| e.seq.wrapping_add(1))
        .unwrap_or(q.after);
    Json(EventsPage {
        events: filtered,
        next_seq,
    })
    .into_response()
}

/// `true` when the client wants the SSE upgrade. Either:
/// - `Accept` contains `text/event-stream`, or
/// - the `?stream=1` query parameter is set.
fn wants_sse(headers: &HeaderMap, stream_flag: Option<u8>) -> bool {
    if matches!(stream_flag, Some(n) if n > 0) {
        return true;
    }
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false)
}

/// Build the live-tail stream. Emits any backlog with `seq > after`,
/// then polls the supervisor's ring every [`POLL_INTERVAL`] and emits
/// new entries.
///
/// The cursor is "the next seq that has not yet been emitted". We filter
/// `seq >= cursor` and, after each emit, advance to `last_seq + 1`. The
/// stream closes when the supervisor's lifecycle reaches `Stopped` /
/// `Failed` *and* the ring has no events left to drain.
fn sse_stream(
    handle: SupervisorHandle,
    after: u64,
) -> impl Stream<Item = Result<SseEvent, Infallible>> {
    use futures::stream::{self, StreamExt};

    // Initial backlog: every event with `seq > after`.
    let initial: Vec<RingEvent> = handle
        .events()
        .into_iter()
        .filter(|e| e.seq > after)
        .collect();
    let initial_cursor = initial
        .last()
        .map(|e| e.seq.wrapping_add(1))
        // No backlog → cursor starts at `after + 1` (the first not-yet-
        // emitted seq). Saturating add keeps us defined at u64::MAX.
        .unwrap_or_else(|| after.saturating_add(1));

    let initial_stream = stream::iter(initial.into_iter().map(to_sse_event));

    // Polling loop yields a `Vec<RingEvent>` per tick; we flatten it
    // into one event per Stream item. `None` terminates the stream.
    let poll_stream = stream::unfold(
        (handle, initial_cursor),
        move |(handle, cursor)| async move {
            loop {
                tokio::time::sleep(POLL_INTERVAL).await;
                let state = *handle.state().borrow();
                let snapshot = handle.events();
                let new: Vec<RingEvent> =
                    snapshot.into_iter().filter(|e| e.seq >= cursor).collect();
                if !new.is_empty() {
                    let next_cursor = new
                        .last()
                        .map(|e| e.seq.wrapping_add(1))
                        .unwrap_or(cursor);
                    return Some((new, (handle, next_cursor)));
                }
                if matches!(
                    state,
                    starter_ext_spi::LifecycleState::Stopped
                        | starter_ext_spi::LifecycleState::Failed
                ) {
                    return None;
                }
            }
        },
    )
    .flat_map(|batch| stream::iter(batch.into_iter().map(to_sse_event)));

    initial_stream.chain(poll_stream)
}

fn to_sse_event(e: RingEvent) -> Result<SseEvent, Infallible> {
    let json = serde_json::to_string(&e).unwrap_or_else(|_| "{}".to_string());
    Ok(SseEvent::default().id(e.seq.to_string()).data(json))
}
