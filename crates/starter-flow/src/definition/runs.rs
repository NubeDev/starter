//! `RunRegistry` — the bookkeeping the HR-4 `apply_policy` dispatch
//! needs to know which in-flight [`RunCancel`] handles to fire when
//! a structural swap lands.
//!
//! Per `DOCS/flow/scope/hot-reload.md` HR4 (`Restart` and the
//! `LiveMigrate → Restart` fallback paths), the publish chokepoint
//! must cancel every run executing against the *previous* revision
//! when the operator's policy says so. The engine itself does not
//! track per-run handles today (`FlowRunner` is per-run), so this
//! module owns the small `(FlowId, FlowRevisionId) → [Weak<RunCancel>]`
//! map and the lifecycle hooks the runner calls on start / finish.
//!
//! Design notes:
//!
//! - Handles are stored as [`Weak`] so a runner that drops its
//!   handle without calling [`RunRegistration::release`] does not
//!   keep the registry growing — a subsequent
//!   [`RunRegistry::cancel_for`] simply observes the dead `Weak` and
//!   prunes it.
//! - Registration returns a typed RAII guard ([`RunRegistration`])
//!   so the common case ("register on start, deregister on finish")
//!   doesn't need an explicit `unregister` call. The runner keeps
//!   the guard alive for the lifetime of the run.
//! - The lock is a synchronous `Mutex` because every operation is
//!   O(1) (or O(runs-on-this-revision), tiny) and holding it across
//!   an `await` would gain us nothing. The registry is never read
//!   on the propagator hot path.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use starter_flow_spi::flow::{FlowId, FlowRevisionId};

use crate::run::RunCancel;

/// Bookkeeping registry that maps `(flow, revision) → [run cancels]`.
///
/// Only used by [`super::manager::DefinitionManager`] for the HR-4
/// `Restart` / `LiveMigrate` dispatch path. Lives in its own
/// module to keep the manager's surface area small.
#[derive(Default, Debug)]
pub struct RunRegistry {
    inner: Mutex<HashMap<(FlowId, FlowRevisionId), Vec<Weak<RunCancel>>>>,
}

impl RunRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an in-flight run's [`RunCancel`] under
    /// `(flow, revision)`. Returns a guard that removes the
    /// registration when dropped (or, if the run finishes before
    /// drop, the caller can call [`RunRegistration::release`]
    /// explicitly).
    ///
    /// Multiple concurrent runs for the same `(flow, revision)`
    /// pair are supported — they accumulate as a `Vec`.
    pub fn register(
        self: &Arc<Self>,
        flow: FlowId,
        revision: FlowRevisionId,
        cancel: Arc<RunCancel>,
    ) -> RunRegistration {
        let weak = Arc::downgrade(&cancel);
        let key = (flow.clone(), revision);
        {
            let mut guard = self.inner.lock().expect("RunRegistry mutex poisoned");
            guard.entry(key.clone()).or_default().push(weak.clone());
        }
        RunRegistration {
            registry: Arc::downgrade(self),
            key,
            cancel: weak,
            released: false,
        }
    }

    /// Fire `cancel()` on every registered, still-live
    /// [`RunCancel`] for the `(flow, revision)` pair and return
    /// the count actually cancelled (a `Weak` whose `RunCancel`
    /// has already been dropped contributes zero).
    ///
    /// The entry is removed atomically with the cancel walk so a
    /// second call returns `0`.
    pub fn cancel_for(&self, flow: &FlowId, revision: &FlowRevisionId) -> usize {
        let entries = {
            let mut guard = self.inner.lock().expect("RunRegistry mutex poisoned");
            guard.remove(&(flow.clone(), *revision)).unwrap_or_default()
        };
        let mut cancelled = 0usize;
        for weak in entries {
            if let Some(cancel) = weak.upgrade() {
                cancel.cancel();
                cancelled += 1;
            }
        }
        cancelled
    }

    /// Number of `(flow, revision)` entries currently tracked. Used
    /// by tests; production code has no use for the global count.
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("RunRegistry mutex poisoned")
            .len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn drop_registration(&self, key: &(FlowId, FlowRevisionId), cancel: &Weak<RunCancel>) {
        let mut guard = self.inner.lock().expect("RunRegistry mutex poisoned");
        let cancel_ptr = cancel.as_ptr();
        if let Some(entry) = guard.get_mut(key) {
            entry.retain(|w| w.as_ptr() != cancel_ptr);
            if entry.is_empty() {
                guard.remove(key);
            }
        }
    }
}

/// RAII guard returned by [`RunRegistry::register`]. Removes the
/// registered weak handle on drop so a finished run frees its
/// registry slot without an explicit unregister call.
///
/// `release()` consumes the guard for callers that want to deregister
/// explicitly before drop (e.g. tests that assert post-finish
/// registry state).
#[must_use = "drop the registration when the run finishes"]
pub struct RunRegistration {
    registry: Weak<RunRegistry>,
    key: (FlowId, FlowRevisionId),
    cancel: Weak<RunCancel>,
    released: bool,
}

impl RunRegistration {
    /// Explicitly remove the registration. Equivalent to dropping
    /// the guard; provided so call sites can be loud about the
    /// deregister point.
    pub fn release(mut self) {
        self.do_release();
    }

    fn do_release(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        if let Some(registry) = self.registry.upgrade() {
            registry.drop_registration(&self.key, &self.cancel);
        }
    }
}

impl Drop for RunRegistration {
    fn drop(&mut self) {
        self.do_release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_flow_spi::Cancel;

    fn fid(s: &str) -> FlowId {
        FlowId::new(s).unwrap()
    }

    #[test]
    fn register_and_cancel_fires_handle() {
        let registry = Arc::new(RunRegistry::new());
        let cancel = RunCancel::new();
        let rev = FlowRevisionId::new();
        let _guard = registry.register(fid("hr4.test"), rev, cancel.clone());

        assert_eq!(registry.cancel_for(&fid("hr4.test"), &rev), 1);
        assert!(cancel.is_cancelled());
        // Second call is a no-op (entry was removed).
        assert_eq!(registry.cancel_for(&fid("hr4.test"), &rev), 0);
    }

    #[test]
    fn drop_guard_removes_registration() {
        let registry = Arc::new(RunRegistry::new());
        let cancel = RunCancel::new();
        let rev = FlowRevisionId::new();
        {
            let _guard = registry.register(fid("hr4.test"), rev, cancel.clone());
            assert_eq!(registry.len(), 1);
        }
        assert!(registry.is_empty());
        // Cancel call now finds nothing.
        assert_eq!(registry.cancel_for(&fid("hr4.test"), &rev), 0);
        assert!(!cancel.is_cancelled());
    }

    #[test]
    fn cancel_skips_dropped_handles() {
        let registry = Arc::new(RunRegistry::new());
        let rev = FlowRevisionId::new();
        let cancel = RunCancel::new();
        let mut guard = Some(registry.register(fid("hr4.test"), rev, cancel.clone()));
        // Forget the guard so the entry stays in the map even after
        // the cancel is dropped.
        std::mem::forget(guard.take());
        drop(cancel);
        assert_eq!(registry.cancel_for(&fid("hr4.test"), &rev), 0);
    }

    #[test]
    fn multiple_runs_per_revision_all_cancel() {
        let registry = Arc::new(RunRegistry::new());
        let rev = FlowRevisionId::new();
        let c1 = RunCancel::new();
        let c2 = RunCancel::new();
        let _g1 = registry.register(fid("hr4.multi"), rev, c1.clone());
        let _g2 = registry.register(fid("hr4.multi"), rev, c2.clone());
        assert_eq!(registry.cancel_for(&fid("hr4.multi"), &rev), 2);
        assert!(c1.is_cancelled());
        assert!(c2.is_cancelled());
    }
}
