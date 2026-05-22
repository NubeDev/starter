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

mod registry;
pub mod routes;
mod service;

pub use registry::ReversibleRegistry;
pub use routes::{undo_router, UndoResponse};
pub use service::{InMemoryUndoCursor, UndoCursor, UndoService};
