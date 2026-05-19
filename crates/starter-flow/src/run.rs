//! Run lifecycle: `Cancel` plumbing + `RunState` + checkpointing per R6.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" (lifecycle +
//! Cancel propagation) and "Phase 7 — three-level stop" (checkpoint
//! persistence on Pause / Stopped). Owns the per-`RunId` handle the
//! engine hands back to callers and the checkpoint serializer that
//! writes through `RunStore`.
//!
//! Phase-2 stage 4 (propagator): only the per-run [`RunCancel`]
//! handle lands here today — the propagator needs a concrete
//! [`Cancel`] impl to wire the run-wide cancel signal through to the
//! main propagation loop and into every `NodeBehavior::invoke` call.
//! The full `RunState` + checkpoint serializer + `RunStore` writer
//! arrive in later stages of Phase 2 / Phase 7.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

use starter_flow_spi::Cancel;

/// Per-run cancellation handle.
///
/// SCOPE R13 — cancellation across the flow engine reuses the existing
/// [`Cancel`] seam. The propagator awaits [`Cancel::cancelled`] in its
/// main `select!` so a fired cancel stops scheduling further hops
/// within a few hundred milliseconds; every `NodeBehavior::invoke`
/// receives a borrow of this handle through `NodeCtx` so node bodies
/// can abort their own work too.
///
/// The implementation is a plain `AtomicBool` plus a [`Notify`] — no
/// `tokio_util` dependency leaks into this crate. Construction goes
/// through [`Self::new`] which returns an [`Arc`] because the engine
/// hands the same handle to (a) the propagator task, (b) every
/// in-flight node invocation, and (c) the public run handle the
/// engine's caller can flip.
#[derive(Debug, Default)]
pub struct RunCancel {
    flag: AtomicBool,
    notify: Notify,
}

impl RunCancel {
    /// Construct a fresh, un-cancelled run cancel handle.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Flip the cancel flag and wake every waiter. Idempotent — a
    /// second call is a no-op.
    pub fn cancel(&self) {
        if !self.flag.swap(true, Ordering::SeqCst) {
            self.notify.notify_waiters();
        }
    }
}

impl Cancel for RunCancel {
    fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            if self.flag.load(Ordering::SeqCst) {
                return;
            }
            loop {
                // Register the waiter *before* re-checking the flag so
                // we don't miss a `notify_waiters()` racing with our
                // load.
                let notified = self.notify.notified();
                if self.flag.load(Ordering::SeqCst) {
                    return;
                }
                notified.await;
                if self.flag.load(Ordering::SeqCst) {
                    return;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn cancel_flips_and_wakes_waiters() {
        let c = RunCancel::new();
        assert!(!c.is_cancelled());

        let c2 = c.clone();
        let waiter = tokio::spawn(async move {
            c2.cancelled().await;
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        c.cancel();

        timeout(Duration::from_millis(200), waiter)
            .await
            .expect("waiter never woke")
            .expect("waiter task panicked");
        assert!(c.is_cancelled());
    }

    #[tokio::test]
    async fn cancelled_returns_immediately_if_already_cancelled() {
        let c = RunCancel::new();
        c.cancel();
        timeout(Duration::from_millis(50), c.cancelled())
            .await
            .expect("cancelled() did not resolve immediately for an already-cancelled token");
    }
}
