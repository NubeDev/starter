//! SSE handlers for `/api/marts/events` and `/api/entities/events`.
//! Both use starter-server's `sse::keep_alive` (15 s).

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::routing::get;
use axum::Router;
use futures::stream::{self, Stream};
use starter_server::sse::keep_alive;

use crate::nodes::runtime::WarehouseRuntime;

pub fn sse_router() -> Router<Arc<WarehouseRuntime>> {
    Router::new()
        .route("/api/marts/events", get(mart_events))
        .route("/api/entities/events", get(entity_events))
}

async fn mart_events(
    State(_rt): State<Arc<WarehouseRuntime>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::empty::<Result<Event, Infallible>>();
    Sse::new(stream).keep_alive(keep_alive())
}

async fn entity_events(
    State(_rt): State<Arc<WarehouseRuntime>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::empty::<Result<Event, Infallible>>();
    Sse::new(stream).keep_alive(keep_alive())
}
