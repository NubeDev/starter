//! Reusable building blocks for Tauri 2 desktop shells built on top of
//! the `starter-*` crates. Extracted from real production shells — the
//! patterns here repeat verbatim across every Tauri app, so they live
//! here instead of being copy-pasted.
//!
//! Modules:
//!
//! - [`paths`] — `~/.<app>/workspaces/<slug>/…` layout with deterministic
//!   per-folder slugs. Always available.
//! - [`error`] — serde-friendly `CommandError`/`CommandResult` so a
//!   `#[tauri::command]` can return any `Display` error to the frontend.
//!   Requires the `ipc` feature.
//! - [`events`] — bridges a `futures::Stream` of events to a
//!   `tauri::ipc::Channel`, with per-channel cancellation. Requires the
//!   `ipc` feature.
//! - [`rest`] — bind a loopback REST server on an ephemeral port and
//!   surface the bound URL to the UI. Requires the `ipc` feature.

pub mod paths;

#[cfg(feature = "ipc")]
pub mod error;

#[cfg(feature = "ipc")]
pub mod events;

#[cfg(feature = "ipc")]
pub mod rest;
