//! Adapt a `futures::Stream` of serializable items into an SSE
//! response with the project-standard keep-alive policy.

use std::convert::Infallible;

use axum::response::sse::{Event, Sse};
use futures::Stream;
use serde::Serialize;

use super::keep_alive::keep_alive;

/// Wrap a stream of `T: Serialize` items as a JSON-encoded SSE
/// response.
///
/// Each item becomes a single `data:` event. Encoding failures are
/// dropped silently — they're a programming error in the producer,
/// not a runtime condition.
pub fn from_stream<S, T>(stream: S) -> Sse<impl Stream<Item = Result<Event, Infallible>>>
where
    S: Stream<Item = T> + Send + 'static,
    T: Serialize + Send + 'static,
{
    use futures::StreamExt;

    let events = stream.map(|item| {
        let json = serde_json::to_string(&item).unwrap_or_else(|_| "{}".to_string());
        Ok(Event::default().data(json))
    });

    Sse::new(events).keep_alive(keep_alive())
}
