//! Cancellation plumbing shared by the runners.
//!
//! ArkFlow's `Stream::run` takes a `CancellationToken` and aborts an in-flight
//! `input.read()` via `tokio::select!` when it fires (this is the git-HEAD
//! signature the engine is pinned to). The runners fire that token on three
//! events: a wall-clock deadline, a breached collector cap, and — for the live
//! path — client disconnect.

use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Spawn a task that cancels `token` after `budget`. Returns the handle so the
/// caller can abort the timer once the stream finishes on its own, avoiding a
/// lingering task. A `None` budget installs no timer.
pub fn deadline(token: CancellationToken, budget: Option<Duration>) -> Option<JoinHandle<()>> {
    let budget = budget?;
    Some(tokio::spawn(async move {
        tokio::time::sleep(budget).await;
        token.cancel();
    }))
}
