//! # starter-server
//!
//! axum app builder. The consumer constructs one or more
//! `axum::Router<AppState>` with their own routes and hands them to
//! [`builder::ServerBuilder`]; this crate merges them, mounts the
//! starter-owned health / metrics / OpenAPI routes, wires middleware,
//! and binds the listener.
//!
//! See SCOPE.md "starter-server" for the seam shape. The Router seam
//! is deliberately a real `axum::Router`, not a `Route` trait —
//! that's how axum wants to be composed.
//!
//! Layout:
//!
//! - [`builder`] — `ServerBuilder` + configured listener bind.
//! - [`routes`] — starter-owned routes (`/health`, `/metrics`,
//!   `/openapi.json`).
//! - [`sse`] — SSE helpers consumers use to expose streaming
//!   endpoints.
//! - [`openapi`] — utoipa document assembly.
//! - [`error`] — mapping `starter_spi::Error` → HTTP `Problem` body.
//! - [`testing`] — opt-in test harness (`feature = "testing"`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod auth;
pub mod builder;
pub mod error;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod sse;
pub mod static_assets;

#[cfg(feature = "testing")]
pub mod testing;

pub use builder::ServerBuilder;
