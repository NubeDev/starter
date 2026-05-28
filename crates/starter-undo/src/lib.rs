//! # starter-undo
//!
//! Per-actor undo/redo over the changelog. The [`UndoService`]
//! groups changes by `group_id`, dispatches through a
//! [`ReversibleRegistry`], and keeps a per-actor redo stack so the
//! second `undo` walks past the row that was just undone.
//!
//! See `DOCS/backend/undo-redo/SCOPE.md` §"Feature mapping".

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod actor_local;
pub mod dispatch;
mod registry;
pub mod routes;
mod service;

#[cfg(feature = "sqlite")]
pub mod cursor_sqlite;

#[cfg(feature = "postgres")]
pub mod cursor_postgres;

pub use dispatch::{record_if_reversible, ChangeDraft};
pub use registry::ReversibleRegistry;
pub use routes::{undo_router, UndoApi, UndoResponse};
pub use service::{actor_key, InMemoryUndoCursor, UndoCursor, UndoService};

/// Top-level convenience wrapper around [`UndoService::undo`] —
/// the verb the `rubix.undo.last` tool dispatches.
///
/// `_scope` is reserved for a forthcoming per-resource undo filter
/// (Goals 2/3/4 will surface it). The current implementation ignores
/// it and walks the actor's most recent group; the parameter is here
/// so callers can be wired today without a follow-up signature
/// change.
pub async fn undo_last(
    service: &UndoService,
    actor: &starter_spi::changelog::Actor,
    _scope: Option<&starter_spi::authz::ResourceRef>,
) -> starter_spi::Result<starter_spi::changelog::GroupId> {
    service.undo(actor).await
}

/// Top-level convenience wrapper around [`UndoService::redo`] —
/// the verb the `rubix.undo.redo` tool dispatches.
///
/// `_scope` mirrors [`undo_last`]'s reserved per-resource filter so
/// the two verbs keep the same client contract.
pub async fn redo_last(
    service: &UndoService,
    actor: &starter_spi::changelog::Actor,
    _scope: Option<&starter_spi::authz::ResourceRef>,
) -> starter_spi::Result<starter_spi::changelog::GroupId> {
    service.redo(actor).await
}
