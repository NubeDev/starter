//! REST surface for notes. A normal axum `Router` that gets merged
//! into the starter-server builder alongside starter's own routes
//! (`/health`, `/metrics`, `/openapi.json`, `/auth/claim`, `/mcp`).
//!
//! Consumer-defined routes. Starter does not know what a "note" is.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use futures::StreamExt;
use starter_spi::auth::Principal;
use tokio::sync::broadcast;
use utoipa::OpenApi;

use crate::domain::{CreateNote, Note, NoteError, NoteStore};

#[derive(Clone)]
pub struct NotesState {
    pub store: Arc<NoteStore>,
    /// Fan-out for SSE subscribers. Every successful POST /notes
    /// publishes here. `broadcast` drops events for slow subscribers
    /// (lagging clients see a `Lagged` error in the stream) — that's
    /// the right shape for "real-time but eventually-consistent" UIs.
    pub events: broadcast::Sender<Note>,
}

#[derive(OpenApi)]
#[openapi(
    paths(create_note, list_notes, get_note, delete_note, me, stream_notes),
    components(schemas(Note, CreateNote))
)]
pub struct NotesApi;

/// Build the consumer's notes router. Generic over the parent state
/// type `S` so it composes into any `Router<S>` the consumer hands
/// to `ServerBuilder`.
pub fn notes_router<S>(state: NotesState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/notes", post(create_note).get(list_notes))
        .route("/notes/{id}", get(get_note).delete(delete_note))
        .route("/notes/stream", get(stream_notes))
        .route("/auth/me", get(me))
        .with_state(state)
}

/// `GET /notes/stream` — Server-Sent Events. Each new note POSTed
/// elsewhere appears here as a `data: { ... }` JSON event. Demonstrates
/// the `starter_server::sse::from_stream` helper: consumer brings a
/// `Stream<Item = T: Serialize>`, starter handles SSE framing + the
/// standard 15-second keep-alive policy.
#[utoipa::path(
    get,
    path = "/notes/stream",
    tag = "notes",
    operation_id = "stream_notes",
    responses(
        (status = 200, description = "SSE stream of new notes", content_type = "text/event-stream"),
        (status = 401, description = "Missing bearer"),
    ),
)]
async fn stream_notes(
    State(s): State<NotesState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    if principal.is_none() {
        return (StatusCode::UNAUTHORIZED, "unauthenticated").into_response();
    }
    let rx = s.events.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        // Drop `Lagged` errors; only emit successfully-received notes.
        .filter_map(|res| async move { res.ok() });
    starter_server::sse::from_stream(stream).into_response()
}

#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    operation_id = "me",
    responses(
        (status = 200, description = "Current principal"),
        (status = 401, description = "Missing bearer"),
    ),
)]
async fn me(principal: Option<Extension<Principal>>) -> impl IntoResponse {
    match principal {
        Some(Extension(p)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "subject": p.subject,
                "email": "",
                "role": format!("{:?}", p.role).to_lowercase(),
            })),
        )
            .into_response(),
        None => (StatusCode::UNAUTHORIZED, "unauthenticated").into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/notes",
    tag = "notes",
    operation_id = "create_note",
    request_body = CreateNote,
    responses(
        (status = 201, description = "Note created", body = Note),
        (status = 401, description = "Missing bearer"),
    ),
)]
async fn create_note(
    State(s): State<NotesState>,
    principal: Option<Extension<Principal>>,
    Json(body): Json<CreateNote>,
) -> impl IntoResponse {
    let Some(Extension(p)) = principal else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthenticated"})),
        )
            .into_response();
    };
    match s.store.create(&body.body, &p.subject).await {
        Ok(note) => {
            // Send-error means no subscribers — that's fine, the
            // broadcast channel is fire-and-forget.
            let _ = s.events.send(note.clone());
            (
                StatusCode::CREATED,
                Json(serde_json::to_value(&note).unwrap()),
            )
                .into_response()
        }
        Err(e) => internal(e),
    }
}

#[utoipa::path(
    get,
    path = "/notes",
    tag = "notes",
    operation_id = "list_notes",
    responses((status = 200, description = "All notes", body = [Note])),
)]
async fn list_notes(
    State(s): State<NotesState>,
    principal: Option<Extension<Principal>>,
) -> impl IntoResponse {
    if principal.is_none() {
        return (StatusCode::UNAUTHORIZED, "unauthenticated").into_response();
    }
    match s.store.list().await {
        Ok(notes) => (StatusCode::OK, Json(serde_json::to_value(&notes).unwrap())).into_response(),
        Err(e) => internal(e),
    }
}

#[utoipa::path(
    get,
    path = "/notes/{id}",
    tag = "notes",
    operation_id = "get_note",
    params(("id" = String, Path, description = "Note id")),
    responses(
        (status = 200, description = "The note", body = Note),
        (status = 404, description = "Not found"),
    ),
)]
async fn get_note(
    State(s): State<NotesState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if principal.is_none() {
        return (StatusCode::UNAUTHORIZED, "unauthenticated").into_response();
    }
    match s.store.get(&id).await {
        Ok(note) => (StatusCode::OK, Json(serde_json::to_value(&note).unwrap())).into_response(),
        Err(NoteError::NotFound(_)) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => internal(e),
    }
}

#[utoipa::path(
    delete,
    path = "/notes/{id}",
    tag = "notes",
    operation_id = "delete_note",
    params(("id" = String, Path, description = "Note id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
)]
async fn delete_note(
    State(s): State<NotesState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if principal.is_none() {
        return (StatusCode::UNAUTHORIZED, "unauthenticated").into_response();
    }
    match s.store.delete(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(NoteError::NotFound(_)) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => internal(e),
    }
}

fn internal(e: impl std::fmt::Display) -> axum::response::Response {
    tracing::error!(error = %e, "notes route error");
    (StatusCode::INTERNAL_SERVER_ERROR, "internal").into_response()
}
