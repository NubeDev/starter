//! REST adapter — Adapter Phase 5 (SCOPE R13).
//!
//! The REST surface a `starter-extensions` host mounts alongside the
//! admin slice. Composed from two independent contribute streams in the
//! manifest:
//!
//! - `contributes.rest[]` — per-extension route fragments at the method
//!   and path the manifest declares. Adapters from different extensions
//!   are merged into one [`axum::Router`]; a path/method collision
//!   between two extensions is a **load-time error** (returned from
//!   [`rest_router`]), never a silent route shadowing.
//! - `contributes.tools[]` — one auto-mounted `POST /tools/<id>` per
//!   tool entry. R13: one tool reaches both MCP and REST without the
//!   extension author writing two handlers.
//!
//! Cross-cutting concerns the adapter owns (the extension never touches
//! them):
//!
//! - **Per-entry auth** — `AuthGate { require_role, require_scope }` on
//!   each entry is enforced as an outer middleware layer (`with_role`
//!   and `with_scope` from `starter-server`); the extension only sees
//!   requests that passed the gate.
//! - **Input-schema validation** — the manifest's `request_schema`
//!   (REST) / `input_schema` (tools) is loaded once at build time and
//!   each request body is checked against it before the dispatcher is
//!   invoked. The check is a small structural pass (type + required) —
//!   sufficient for v0.1 and zero new dependencies.
//! - **Streaming render** — `streaming: sse` produces a
//!   `text/event-stream` response with a `retry:` header + 15s
//!   heartbeat; `streaming: ndjson` produces newline-delimited
//!   `application/x-ndjson`. Both fire a `stream.cancel` toward the
//!   extension within a few hundred milliseconds of the client closing
//!   the connection (the "Streaming-response-cancels-promptly" smoke
//!   test in `tests/rest_streaming.rs` enforces this).

mod auth;
mod cache;
mod dispatcher;
mod handler;
mod router;
mod schema;

pub use cache::{DispatcherCache, KindCacheRegistry, OrphanSidecar, SidecarLoadError};
pub use dispatcher::{
    BuiltinRestDispatcher, DispatchError, NotWiredDispatcher, ProcessRestDispatcher,
    RestDispatcher, StreamResponse,
};
pub use router::{rest_router, RestBuildError, RestRouterOptions};
pub use schema::SchemaCheck;
