//! Per-actor undo / redo (WS-12 §3.6 backend).
//!
//! nexus needs its **own** undo routes rather than the starter
//! [`starter_undo::undo_router`]: that router shares one global
//! `Arc<UndoService>`, but a nexus [`starter_undo::UndoService`] is *per tenant*
//! (its [`nexus_store::changelog::NexusChangeLog`] binds `app.tenant_id`). So
//! these handlers build the service per request from the boot-shared registry +
//! redo cursor and the caller's tenant pin, then undo/redo the caller's own most
//! recent group. The response echoes the `group_id` applied so the UI can refresh
//! the affected resources.

pub mod apply;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

/// `/api/v1/undo` and `/api/v1/redo`, both targeting the authenticated principal.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/undo", post(apply::undo))
        .route("/api/v1/redo", post(apply::redo))
}
