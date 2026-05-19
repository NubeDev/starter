//! # starter-ext-workers — Adapter Phase 7 (SCOPE R13)
//!
//! Periodic scheduler that invokes every `contributes.workers` entry's
//! handler at the manifest-declared cadence:
//!
//! - `interval_seconds` — nominal period between consecutive runs.
//! - `jitter_seconds` (optional) — uniform random spread added on top.
//! - `on_error.retry` (`exponential` | `never`) — backoff strategy when
//!   the handler returns `Err`. Bounds: `initial_backoff_ms` /
//!   `max_backoff_ms`. Capped by `max_attempts` consecutive failures
//!   before the worker stops scheduling new runs.
//!
//! Workers are **periodic, not jobs**: there is no shared queue and
//! no fan-out across hosts. Each worker owns its own task in the
//! scheduler. This mirrors the parent SCOPE non-goal — extensions
//! contribute scheduled work, not durable jobs.
//!
//! State the scheduler maintains per worker is surfaced to operators
//! through [`WorkerStateSource`], which `starter-ext-server` reads when
//! a client hits `GET /extensions/<id>`:
//!
//! - `last_run` — wall-clock time of the most recent attempt.
//! - `last_error` — sticky error from the most recent failing run
//!   (cleared on the next success).
//! - `next_due` — wall-clock time of the next scheduled run, or `None`
//!   if the worker has stopped (max attempts reached, `retry: never`,
//!   or the extension is disabled).
//! - `attempt` — consecutive failure count (zero after a success).
//! - `state` — coarse [`WorkerStatus`] for the admin UI.
//!
//! Testing seam: [`WorkersScheduler::tick_now`] forces an immediate
//! invocation of a single worker, ignoring `next_due`. Deterministic
//! tests use it to drive failure / recovery transitions without
//! `tokio::time::sleep` paranoia.
//!
//! ## v0.1 flavour coverage
//!
//! - **Builtin** — handler closures registered through
//!   [`BuiltinWorkerRegistry`]. Runs on the tokio blocking pool so a
//!   slow handler does not park the scheduler runtime.
//! - **Process / wasm** — wired through [`WorkerDispatcher`], which the
//!   v0.1 stubs answer with `WorkerError::NotWired`. The synchronous
//!   JSON-RPC dispatch slice will fill those in additively without
//!   touching the scheduler.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod dispatcher;
mod scheduler;
mod state;

pub mod testing;

pub use dispatcher::{
    BuiltinWorkerDispatcher, BuiltinWorkerRegistry, NotWiredWorkerDispatcher,
    ProcessWorkerDispatcher, WasmWorkerDispatcher, WorkerDispatcher, WorkerError, WorkerHandler,
    DEFAULT_WORKER_TIMEOUT,
};
pub use scheduler::{SchedulerOptions, WorkersScheduler, WorkersSchedulerHandle};
pub use state::{WorkerState, WorkerStateSource, WorkerStatus};
