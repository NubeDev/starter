//! Audit / undo DTOs (WS-12).
//!
//! The audit read endpoints return the platform `ChangePage`/`Change` types
//! (re-exported from `starter-changelog`, already part of the OpenAPI contract),
//! so the only net-new wire type here is the undo/redo response.

mod forget;
mod undo;

pub use forget::{ForgetRequest, ForgetResponse};
pub use undo::UndoResponse;
