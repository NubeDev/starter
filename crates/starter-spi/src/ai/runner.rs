//! `AiRunner` — the seam over a single AI provider.
//!
//! Trait shape lifted from `codeless-workspace/ai-runner` (SCOPE q7).
//! The `Cancel` trait is local so this crate does not pull
//! `tokio_util`; the concrete impl in `starter-ai` wraps
//! `tokio_util::sync::CancellationToken`.

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::event::Event;
use super::input::RunnerInput;
use super::provider::Provider;
use super::result::{RunResult, RunnerError};
use super::session::SessionId;

/// Cancellation handle a caller can flip to abort an in-flight run.
///
/// Both shapes are provided so runners can integrate with whichever
/// pattern fits their loop: REST runners select against
/// `cancelled().await` inside `tokio::select!`; CLI runners typically
/// poll `is_cancelled()` between stdout reads.
pub trait Cancel: Send + Sync + 'static {
    /// Returns `true` once the caller has requested cancellation.
    fn is_cancelled(&self) -> bool;

    /// A future that resolves when cancellation is requested. Lifetime
    /// is bound to `&self` so the implementor can hold internal state
    /// without an `Arc`.
    fn cancelled<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>;
}

/// Streaming-event channel handed to `AiRunner::run`.
///
/// **Backpressure semantics.** The channel is bounded; capacity is the
/// caller's choice when constructing the underlying `mpsc::channel`.
///
/// - REST runners drive the run from `async` context and `await` each
///   `send`. A slow consumer slows the producer naturally.
/// - CLI runners emit events from the sync callback the wrapper hands
///   them. Those callbacks cannot `.await`, so CLI runners use
///   `try_send` and drop events when the channel is full. Drops are
///   best-effort: a `tracing::warn!` records the overflow so an
///   under-sized channel is visible in logs.
pub type OnEvent = mpsc::Sender<Event>;

/// One AI provider, runnable.
#[async_trait]
pub trait AiRunner: Send + Sync + 'static {
    /// Which provider this runner is for. Used by `Registry` to route
    /// requests.
    fn provider(&self) -> &Provider;

    /// Readiness probe: is the backend installed / configured well
    /// enough to be worth dispatching to? Cheap; called frequently by
    /// health checks.
    ///
    /// CLI runners locate their binary the same way the real run does.
    /// REST runners check credential reachability — they do **not**
    /// make network calls.
    async fn ready(&self) -> bool;

    /// Drive one run end to end. Events stream out through `on_event`;
    /// `cancel` is polled so the runner can abort subprocess / HTTP
    /// body promptly.
    async fn run(
        &self,
        input: RunnerInput,
        session_id: SessionId,
        on_event: OnEvent,
        cancel: &dyn Cancel,
    ) -> Result<RunResult, RunnerError>;
}
