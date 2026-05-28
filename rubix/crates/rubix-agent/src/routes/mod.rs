//! REST routers mounted by the rubix-agent binary.
//!
//! One verb-per-file under this module. The binary's `main.rs` is
//! pure wiring (R5): it calls the per-verb router builder, merges
//! the result into a single axum [`Router`](axum::Router), and
//! hands the assembly to [`crate::health::serve`]. No domain logic
//! lives here. See
//! [docs/design/tools/](../../docs/design/tools/README.md).

pub mod admin;
pub mod auth;
pub mod chat_stream;
pub mod dashboard_events;
pub mod flow_events;
pub mod flow_run;
pub mod openapi_doc;
pub mod registrar;
pub mod stream_frames;
pub mod tools;

pub use registrar::{catalog_to_openapi, OpenApiInfo, RouteEntry, RouteMeta, RouteRegistrar};
pub use stream_frames::{frame_to_sse, StreamFrame};
