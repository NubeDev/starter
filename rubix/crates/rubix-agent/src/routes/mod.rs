//! REST routers mounted by the rubix-agent binary.
//!
//! One verb-per-file under this module. The binary's `main.rs` is
//! pure wiring (R5): it calls the per-verb router builder, merges
//! the result into a single axum [`Router`](axum::Router), and
//! hands the assembly to [`crate::health::serve`]. No domain logic
//! lives here. See
//! [docs/design/tools/](../../docs/design/tools/README.md).

pub mod auth;
pub mod tools;
