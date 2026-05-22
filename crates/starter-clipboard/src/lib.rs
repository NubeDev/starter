//! # starter-clipboard
//!
//! Server-side, principal-scoped clipboard backing copy / paste /
//! duplicate. See `DOCS/backend/undo-redo/SCOPE.md` §"Feature
//! mapping".
//!
//! Today this crate ships an in-memory store. SQLite / Postgres
//! backends (with the HMAC-signed `starter_clipboard` table) land
//! as follow-up crates.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod routes;
mod service;
mod store;

pub use routes::{
    clipboard_router, ClipboardApi, ClipboardRoutesState, CopyRequest, CopyResponse, PasteRequest,
    PasteResponse,
};
pub use service::{ClipboardService, DEFAULT_TTL_SECS};
pub use store::{new_entry, ClipboardEntry, ClipboardStore, InMemoryClipboard};
