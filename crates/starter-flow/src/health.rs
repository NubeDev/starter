//! Engine-level health handle (D-F3.11).
//!
//! SCOPE: Phase 3 "durability hardening" — the engine exposes a
//! lock-free [`EngineHealth`] accessor backed by an `AtomicU8` so the
//! per-run propagator can flip the engine to `Degraded` after five
//! consecutive `RunStore::checkpoint` failures, and so the per-run
//! `FlowRunner::start` path can reject new runs with
//! [`starter_flow_spi::flow::EngineError::BackendUnavailable`] while
//! degraded.
//!
//! The handle is `Clone`able and cheap to share across tasks; every
//! clone observes the same underlying state. The [`Engine`] holds
//! one canonical instance; the per-run [`crate::run::FlowRunner`]
//! receives a clone via [`crate::run::FlowRunner::with_health_handle`]
//! so its [`crate::run::FlowRunner::start`] check and the
//! propagator's "degrade on five failures" transition see the same
//! flag.
//!
//! [`Engine`]: crate::engine::Engine

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use starter_flow_spi::flow::EngineHealth;

const HEALTHY: u8 = 0;
const DEGRADED: u8 = 1;

/// Cloneable, lock-free handle to one engine's [`EngineHealth`] flag.
///
/// Reads use `Ordering::Acquire`; writes use `Ordering::Release` so
/// a reader that observes `Degraded` is guaranteed to also observe
/// the in-memory queue writes that produced the degradation (the
/// queue mutation is sequenced-before the `set_degraded` store).
#[derive(Debug, Clone, Default)]
pub struct HealthHandle {
    inner: Arc<AtomicU8>,
}

impl HealthHandle {
    /// Construct a fresh handle in [`EngineHealth::Healthy`].
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicU8::new(HEALTHY)),
        }
    }

    /// Read the current health.
    pub fn get(&self) -> EngineHealth {
        match self.inner.load(Ordering::Acquire) {
            HEALTHY => EngineHealth::Healthy,
            _ => EngineHealth::Degraded,
        }
    }

    /// Flip to [`EngineHealth::Healthy`]. Idempotent.
    pub fn set_healthy(&self) {
        self.inner.store(HEALTHY, Ordering::Release);
    }

    /// Flip to [`EngineHealth::Degraded`]. Idempotent.
    pub fn set_degraded(&self) {
        self.inner.store(DEGRADED, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_healthy_and_round_trips() {
        let h = HealthHandle::new();
        assert_eq!(h.get(), EngineHealth::Healthy);
        h.set_degraded();
        assert_eq!(h.get(), EngineHealth::Degraded);
        h.set_healthy();
        assert_eq!(h.get(), EngineHealth::Healthy);
    }

    #[test]
    fn clones_observe_the_same_state() {
        let a = HealthHandle::new();
        let b = a.clone();
        a.set_degraded();
        assert_eq!(b.get(), EngineHealth::Degraded);
        b.set_healthy();
        assert_eq!(a.get(), EngineHealth::Healthy);
    }
}
