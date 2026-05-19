//! [`MemoryLimiter`] — the per-call memory cap enforcer.
//!
//! Wraps wasmtime's [`wasmtime::StoreLimits`] in a thin wrapper so the
//! call site reads "build a limiter from `caps.max_memory_bytes`" rather
//! than the (longer) builder pipeline. Decoupling here is mostly
//! cosmetic; the value-add is that the limiter is named after the cap
//! it enforces, so a future reader following the trail from `Caps` finds
//! a one-line indirection rather than a sprawling builder invocation.

use wasmtime::{ResourceLimiter, StoreLimits, StoreLimitsBuilder};

/// Per-store memory limiter.
///
/// One instance per `Store`. The cap applies per linear memory — a
/// multi-memory component is allowed `caps.max_memory_bytes` for each,
/// matching wasmtime's [`StoreLimitsBuilder::memory_size`] semantics.
/// v0.1 has no multi-memory extensions, so the distinction is academic;
/// noting it here so a future minor that does ship one is not surprised.
pub struct MemoryLimiter {
    inner: StoreLimits,
}

impl MemoryLimiter {
    /// Build from the host's [`crate::Caps`].
    pub fn new(max_memory_bytes: usize) -> Self {
        let inner = StoreLimitsBuilder::new()
            .memory_size(max_memory_bytes)
            // Trap on growth failure rather than silently returning -1.
            // The substrate's contract is "the host traps the call when
            // it exceeds the cap"; a silent -1 makes "I ran out of
            // memory" indistinguishable from "I successfully detected
            // that and handled it" at the wasm-level.
            .trap_on_grow_failure(true)
            .build();
        Self { inner }
    }

    /// Borrow as a [`wasmtime::ResourceLimiter`] for
    /// [`wasmtime::Store::limiter`] — the closure takes
    /// `&mut MemoryLimiter` and forwards.
    pub fn as_limiter(&mut self) -> &mut dyn ResourceLimiter {
        &mut self.inner
    }
}

impl std::fmt::Debug for MemoryLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryLimiter").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limiter_constructs_without_panicking() {
        // Smoke test — the limiter itself does not expose a "currently
        // permitted bytes" accessor; we leave the meaningful check to
        // the host crate's integration test where a Store is built.
        let mut lim = MemoryLimiter::new(16 * 1024 * 1024);
        let _ = lim.as_limiter();
    }
}
