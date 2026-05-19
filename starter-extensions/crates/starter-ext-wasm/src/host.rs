//! [`WasmHost`] — the public surface.
//!
//! One `WasmHost` per consumer host. Holds the [`wasmtime::Engine`]
//! (configured for the component model + fuel + epoch interruption) and
//! the per-call [`crate::Caps`] every dispatch call inherits. Behaviour
//! is:
//!
//! - **Stateless per-call instantiation** (SCOPE.md Phase 4 / v0.1
//!   discipline): every [`WasmHost::dispatch_tool`] builds a fresh
//!   `Store<InstanceState>`, a fresh `Linker`, and instantiates the
//!   component once. The instance + store are dropped before the
//!   function returns. The next call starts from zero — no observable
//!   state crosses calls. (`kv` capability, reserved in the WIT
//!   package, is what relaxes this in v0.2.)
//! - **Default-deny linker** (R8): the linker is constructed empty.
//!   Only the WASI categories the call's [`crate::WasiCategorySet`]
//!   names are wired in. A component that imports a WASI interface
//!   the host did not grant fails at `Linker::instantiate` with a
//!   "missing import" error — the link-time enforcement R8 describes.
//! - **Per-call caps** (R8): `Caps::max_fuel` becomes `Store::set_fuel`,
//!   `Caps::max_memory_bytes` is enforced via [`crate::MemoryLimiter`]
//!   installed on `Store::limiter`, and `Caps::wall_clock_deadline` is
//!   translated to a wasmtime epoch deadline (the engine's epoch is
//!   ticked by [`WasmHost::epoch_ticker`] running on a background
//!   task).
//!
//! ## What this stage does not ship
//!
//! Stage 11 lands the wasmtime apparatus. The actual JSON-payload
//! transit through the `starter:extension/guest.dispatch-tool` export
//! depends on either (a) wasmtime's `bindgen!` against
//! `wit/starter-extension.wit` or (b) a hand-rolled `TypedFunc` chain
//! against the component's exported function. Both paths require a
//! matching `wit_bindgen`-bound *guest*; neither is reachable end-to-
//! end until the SDK's wasm.rs glue lands the guest side in a later
//! minor. v0.1 ships the host's call shape so adapters wiring through
//! it have a stable surface; the body returns
//! [`WasmCallOutcome::NotImplemented`] until the guest side ships.

use std::sync::Arc;
use std::time::{Duration, Instant};

use starter_ext_spi::{Error, Result};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

use crate::caps::{Caps, WasiCategory};
use crate::limits::MemoryLimiter;
use crate::state::InstanceState;

/// The outcome of one dispatch call.
///
/// A typed variant per terminal condition rather than a `Result<Vec<u8>,
/// Error>` so the admin endpoint can render "ran out of fuel" without
/// pattern-matching on an error string. The `Error` variant is what an
/// adapter forwards back to a caller; the trap variants are what tracing
/// surfaces.
#[derive(Debug)]
pub enum WasmCallOutcome {
    /// Guest returned `Ok(payload)`. v0.1 always returns an empty Vec
    /// because the typed `dispatch-tool` call is not yet wired (see
    /// the module docs above). Adapters consuming this should match
    /// non-exhaustively so the v0.2 payload is additive.
    Ok {
        /// The opaque JSON payload the guest produced.
        payload: Vec<u8>,
        /// How much fuel the call consumed. Surfaced for tracing /
        /// per-extension quotas.
        fuel_consumed: u64,
        /// Wall-clock time the call took. Useful for catching the
        /// "almost hit the deadline" near-misses.
        elapsed: Duration,
    },
    /// Guest returned `Err(payload)`. Same caveat as `Ok` — payload is
    /// empty in v0.1.
    Err {
        /// The opaque JSON error payload.
        payload: Vec<u8>,
        /// Fuel consumed before the error.
        fuel_consumed: u64,
        /// Wall-clock elapsed.
        elapsed: Duration,
    },
    /// Out-of-fuel trap. Maps to `Error::Capability` at the adapter
    /// boundary (the kernel treats "exhausted compute budget" as a
    /// capability denial — the host gave you N units, you used them
    /// all).
    OutOfFuel,
    /// Epoch deadline exceeded.
    DeadlineExceeded,
    /// Linker rejected a missing WASI import — the component required
    /// a category the host did not grant. SCOPE R8 default-deny.
    MissingImport {
        /// Free-form description of the missing import. wasmtime's
        /// own error message is surfaced verbatim.
        reason: String,
    },
    /// The dispatch-tool guest-export call shape is reserved for the
    /// v0.1+ minor that wires `wit_bindgen` on the guest side. Returned
    /// from `dispatch_tool` today so adapter code can already match
    /// against the full enum.
    NotImplemented,
}

/// One wasm-host instance per consumer host. Cheap to clone (everything
/// inside is `Arc`).
#[derive(Clone)]
pub struct WasmHost {
    engine: Engine,
    caps: Arc<Caps>,
}

impl std::fmt::Debug for WasmHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmHost")
            .field("caps", &self.caps)
            .finish_non_exhaustive()
    }
}

impl WasmHost {
    /// Build a fresh `WasmHost`. Returns `Err(Error::Spawn)` if
    /// wasmtime refuses the configuration — typically because of an
    /// unsupported feature combination on the build (no cranelift,
    /// no component-model). Both are enabled in this crate's pinned
    /// wasmtime build, so a real failure here means the consumer
    /// pulled wasmtime in a second time with conflicting features.
    pub fn new(caps: Caps) -> Result<Self> {
        let mut cfg = Config::new();
        // Component model + WASI-p2 require these explicitly. The
        // pinned wasmtime build defaults `wasm_component_model` to
        // `true`, but the `epoch_interruption` and `consume_fuel`
        // toggles are off by default — they impose runtime overhead
        // the substrate wants paid up front.
        cfg.wasm_component_model(true);
        cfg.consume_fuel(true);
        cfg.epoch_interruption(true);
        // Synchronous embedding — keep `async_support` off so the
        // host's call into `dispatch-tool` is a plain function call
        // and not a wasmtime fiber-based future. Adapters that need
        // streaming surface their own task above the host.
        cfg.async_support(false);
        // Debug info is off — extensions ship optimised, and debug
        // info noticeably enlarges compiled artefacts.
        cfg.debug_info(false);

        let engine = Engine::new(&cfg)
            .map_err(|e| Error::spawn(format!("wasmtime engine init: {e}")))?;
        Ok(Self {
            engine,
            caps: Arc::new(caps),
        })
    }

    /// The wasmtime engine backing this host. Exposed so the consumer
    /// can drive epoch ticks (see [`Self::epoch_ticker`]) from their
    /// own runtime.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// The caps every call inherits. Returned as a borrow because the
    /// caps are immutable for the host's lifetime — tweaking them
    /// without rebuilding the host would leave half the in-flight
    /// calls running with the old values and half with the new.
    pub fn caps(&self) -> &Caps {
        &self.caps
    }

    /// Compile a component from raw bytes (binary or `wat` text). Per
    /// SCOPE.md "Static metadata, never runtime-templated" (R7) the
    /// consumer typically compiles once at load time and caches the
    /// resulting [`Component`]; calling this on every dispatch is
    /// legal but wasteful.
    pub fn compile_component(&self, source: &[u8]) -> Result<Component> {
        Component::new(&self.engine, source)
            .map_err(|e| Error::spawn(format!("wasmtime component compile: {e}")))
    }

    /// Spawn a tokio task that ticks the engine's epoch every
    /// `tick`. The returned future ends when `cancel()` returns
    /// `true` — typically wired to the consumer's `ServerBuilder`
    /// shutdown signal. Without an epoch ticker, the deadline cap
    /// configured in [`Caps`] would never fire.
    ///
    /// `tick` should divide the smallest deadline a consumer expects
    /// to enforce; a `100ms` deadline with a `1s` tick fires at 1s,
    /// not 100ms. A pragmatic default is 10ms.
    pub async fn epoch_ticker<F>(&self, tick: Duration, mut cancel: F)
    where
        F: FnMut() -> bool + Send,
    {
        let engine = self.engine.clone();
        let mut interval = tokio::time::interval(tick);
        // `MissedTickBehavior::Skip` is safer than the default
        // (`Burst`) — a host stalled for a second under load otherwise
        // bursts 100 ticks in a row, which can fire deadlines on
        // unrelated calls that happened to start during the stall.
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if cancel() {
                return;
            }
            engine.increment_epoch();
        }
    }

    /// One dispatch-tool call. Builds a fresh `Store`, applies caps,
    /// instantiates the component, and (in a later minor) invokes the
    /// `starter:extension/guest.dispatch-tool` export. v0.1 returns
    /// [`WasmCallOutcome::NotImplemented`] for the call body while the
    /// linker / instantiation half is fully exercised.
    ///
    /// `categories` overrides the host-level [`Caps::wasi_categories`]
    /// per call so a single host can serve extensions with different
    /// manifests. Adapters typically compute this once at load time
    /// from the manifest's `capabilities:` block and pass it in
    /// verbatim.
    pub fn dispatch_tool(
        &self,
        component: &Component,
        categories: crate::caps::WasiCategorySet,
        _tool_id: &str,
        _params_json: &[u8],
    ) -> Result<WasmCallOutcome> {
        let start = Instant::now();

        // Per-call store. The whole `InstanceState` is dropped when
        // `store` goes out of scope at the end of this function —
        // "stateless per-call instantiation" enforced at the type
        // level rather than as a convention.
        let limiter = MemoryLimiter::new(self.caps.max_memory_bytes);
        let mut store = Store::new(&self.engine, InstanceState::new(limiter));

        // R8: per-call fuel cap. `set_fuel` is fallible if fuel
        // consumption is not enabled on the engine — which `WasmHost::new`
        // does explicitly, so the only path to a real error here is a
        // wasmtime bug.
        store
            .set_fuel(self.caps.max_fuel)
            .map_err(|e| Error::spawn(format!("set_fuel: {e}")))?;

        // R8: per-call deadline. The ticker (`Self::epoch_ticker`)
        // ticks at a fixed cadence; we divide the wall-clock deadline
        // by the ticker's cadence to translate it into "epochs from
        // now". A consumer that has not started the ticker effectively
        // disables the deadline; that is documented behaviour, not a
        // silent default.
        let ticks_beyond = epoch_ticks_for(self.caps.wall_clock_deadline);
        store.set_epoch_deadline(ticks_beyond);

        // R8: per-call memory cap.
        store.limiter(|s| s.limiter.as_limiter());

        // R8: default-deny linker. Construct empty, then add only the
        // granted categories. A component that imports anything else
        // fails `Linker::instantiate` below.
        let linker: Linker<InstanceState> = build_linker(&self.engine, categories)?;

        // Instantiation is where the link-time check fires. Surface
        // missing-import errors as a typed variant so admin tooling
        // can render them without grovelling through error strings.
        let instance = match linker.instantiate(&mut store, component) {
            Ok(i) => i,
            Err(e) => {
                let msg = e.to_string();
                // wasmtime's missing-import diagnostic always contains
                // "unknown import". Detecting it via substring is
                // brittle but stable across wasmtime minors; the
                // alternative — downcasting to a wasmtime-private
                // error type — is no more stable and noisier.
                if msg.contains("unknown import") || msg.contains("imported function") {
                    return Ok(WasmCallOutcome::MissingImport { reason: msg });
                }
                return Err(Error::spawn(format!("wasmtime instantiate: {e}")));
            }
        };

        // The actual `dispatch-tool` typed-call wiring lands with the
        // SDK's wit_bindgen integration; surface a typed
        // `NotImplemented` so adapters compile against the full enum
        // today. The store is dropped on return — per-call statelessness
        // enforced.
        let _ = instance;
        let _ = start; // elapsed is reported only on Ok/Err variants today.
        Ok(WasmCallOutcome::NotImplemented)
    }
}

/// Translate a wall-clock duration into an epoch-tick count, matching
/// the ticker the consumer started via [`WasmHost::epoch_ticker`]. The
/// translation is conservative — we round up to the next tick and add
/// one tick of slack so a deadline of `D` is never observed *before*
/// `D` wall-clock time has actually elapsed.
fn epoch_ticks_for(d: Duration) -> u64 {
    // The host's ticker cadence is `EPOCH_TICK_HZ` ticks per second.
    // Convert milliseconds → ticks. 10ms cadence ⇒ 100Hz.
    const EPOCH_TICK_HZ: u64 = 100;
    let ms = d.as_millis() as u64;
    let tick_ms = 1000 / EPOCH_TICK_HZ;
    ms.div_ceil(tick_ms).saturating_add(1)
}

/// Build the default-deny linker for one call. Granting is additive:
/// every category in `categories` adds one or more `wasi:*` interfaces
/// to the linker; anything not granted stays unimported and fails the
/// instantiation below at the wasmtime-side resolver.
///
/// `wasi:io` is *always* wired — it carries no capability, only
/// resource bookkeeping the WASI-p2 component model itself requires
/// (every component-model resource flows through `wasi:io/streams` and
/// `wasi:io/poll`). A component that did not import them gets no extra
/// surface; one that did but was granted no other category sees an
/// empty input/output stream and zero pollables.
fn build_linker(
    engine: &Engine,
    categories: crate::caps::WasiCategorySet,
) -> Result<Linker<InstanceState>> {
    let mut linker = Linker::<InstanceState>::new(engine);

    // Always-linked: io::error / poll / streams. These satisfy the
    // component-model's resource plumbing without granting any
    // capability — a stream the host never feeds bytes into is just
    // an EOF marker. wasmtime-wasi-io exposes only async versions of
    // `add_to_linker`, so we wire the three sync bindings the
    // wasmtime-wasi crate exports per-interface, mirroring what
    // `wasmtime_wasi::add_to_linker_sync` does internally (lines
    // 456–460 of wasmtime-wasi 30.0.2's src/lib.rs).
    let io_closure = io_state_closure();
    use wasmtime_wasi::bindings;
    bindings::sync::io::poll::add_to_linker_get_host(&mut linker, io_closure)
        .map_err(|e| Error::spawn(format!("wasi:io/poll add: {e}")))?;
    bindings::sync::io::streams::add_to_linker_get_host(&mut linker, io_closure)
        .map_err(|e| Error::spawn(format!("wasi:io/streams add: {e}")))?;

    // Per-category opt-in. Each granted category links exactly the
    // wasmtime-wasi interfaces matching the manifest's category. The
    // bindings module surfaces the per-interface `add_to_linker_get_host`
    // helpers we need; the closure shape (`|t| WasiImpl(IoImpl(t))`)
    // mirrors what `add_to_linker_sync` does internally per interface.
    let wasi_closure = wasi_state_closure();

    for cat in categories.iter() {
        match cat {
            WasiCategory::WallClock => {
                use wasmtime_wasi::bindings;
                bindings::clocks::wall_clock::add_to_linker_get_host(&mut linker, wasi_closure)
                    .map_err(|e| {
                        Error::spawn(format!("wasi:clocks/wall-clock add: {e}"))
                    })?;
                bindings::clocks::monotonic_clock::add_to_linker_get_host(
                    &mut linker,
                    wasi_closure,
                )
                .map_err(|e| Error::spawn(format!("wasi:clocks/monotonic-clock add: {e}")))?;
            }
            WasiCategory::Fs => {
                use wasmtime_wasi::bindings;
                bindings::sync::filesystem::types::add_to_linker_get_host(
                    &mut linker,
                    wasi_closure,
                )
                .map_err(|e| Error::spawn(format!("wasi:filesystem/types add: {e}")))?;
                bindings::filesystem::preopens::add_to_linker_get_host(&mut linker, wasi_closure)
                    .map_err(|e| {
                        Error::spawn(format!("wasi:filesystem/preopens add: {e}"))
                    })?;
            }
            WasiCategory::HttpOut => {
                // `wasi:http` lives in a separate `wasmtime-wasi-http`
                // crate that v0.1 does not pull in to keep the
                // dependency footprint small. A component that
                // imports `wasi:http/outgoing-handler` today gets a
                // `MissingImport` outcome even when `http_out` is
                // declared in the manifest; wiring `wasmtime-wasi-http`
                // is a one-line additive change tracked under
                // SCOPE "What is explicitly out of scope" → revisit
                // when an extension actually needs it.
                tracing::warn!(
                    category = %WasiCategory::HttpOut.as_str(),
                    "wasi:http is reserved for a future minor; the manifest's `http_out` grant has no host-side binding in v0.1",
                );
            }
        }
    }

    Ok(linker)
}

/// The state-projection closure every per-interface `add_to_linker_get_host`
/// wants for a `WasiView`-keyed binding. Identical shape to what
/// `wasmtime-wasi`'s `add_to_linker_sync` uses internally — re-declared
/// here so we can compose it per-category instead of all-at-once.
fn wasi_state_closure(
) -> impl Fn(&mut InstanceState) -> wasmtime_wasi::WasiImpl<&mut InstanceState> + Copy + Send + Sync + 'static
{
    |t| wasmtime_wasi::WasiImpl(wasmtime_wasi::IoImpl(t))
}

/// Same shape as [`wasi_state_closure`] but for the `IoView`-keyed
/// `wasi:io/*` bindings, which take an `IoImpl<...>` rather than a
/// `WasiImpl<...>`. The split is wasmtime-wasi's own; we re-declare
/// the two closures so the type inference at the binding call sites
/// stays unambiguous.
fn io_state_closure(
) -> impl Fn(&mut InstanceState) -> wasmtime_wasi::IoImpl<&mut InstanceState> + Copy + Send + Sync + 'static
{
    |t| wasmtime_wasi::IoImpl(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caps::WasiCategorySet;

    fn test_caps() -> Caps {
        Caps::new(1_000_000, 16 * 1024 * 1024, Duration::from_millis(500))
            .expect("test caps")
    }

    #[test]
    fn host_constructs_with_default_caps() {
        let host = WasmHost::new(test_caps()).expect("engine init");
        assert_eq!(host.caps().max_fuel, 1_000_000);
    }

    #[test]
    fn epoch_ticks_round_up_and_add_slack() {
        // 100Hz ticker ⇒ 10ms per tick. 500ms ⇒ 50 ticks + 1 slack.
        assert_eq!(epoch_ticks_for(Duration::from_millis(500)), 51);
        // 1ms ⇒ 1 tick (round up) + 1 slack.
        assert_eq!(epoch_ticks_for(Duration::from_millis(1)), 2);
        // 0ms (edge case — Caps::new refuses zero, but the helper is
        // pure): div_ceil(0) = 0, plus slack = 1.
        assert_eq!(epoch_ticks_for(Duration::from_millis(0)), 1);
    }

    /// The smoke test SCOPE Phase 4 cares about: instantiate a
    /// minimal component under default-deny and watch the host
    /// accept it (because the component imports nothing).
    #[test]
    fn default_deny_accepts_zero_import_component() {
        let host = WasmHost::new(test_caps()).expect("engine init");
        // A component with no imports and no exports is valid; we use
        // it as the "shape ok, link-time happy" smoke test. The wat
        // form is what the `wat` feature on wasmtime parses.
        let wat = b"(component)";
        let component = host
            .compile_component(wat)
            .expect("trivial empty component compiles");
        let outcome = host
            .dispatch_tool(&component, WasiCategorySet::empty(), "noop", b"{}")
            .expect("dispatch returns Ok wrapper");
        // v0.1 returns NotImplemented because the typed call wiring
        // ships in a later minor; the point of the smoke test is that
        // the linker, the caps, and the instantiation succeed.
        assert!(matches!(outcome, WasmCallOutcome::NotImplemented));
    }

    /// SCOPE R8 "default-deny": the linker stays minimal until grants
    /// are added. We exercise the *linker* construction path directly
    /// here rather than instantiating a WAT-form component that imports
    /// WASI — component-model WAT for typed WASI imports is verbose
    /// enough that the test would obscure the property it asserts.
    /// The end-to-end "real WASI import refused" smoke test lands with
    /// the SDK's wit_bindgen integration in a later minor (the example
    /// crate `hello-wasm` already exercises the build wiring; what's
    /// missing is a precompiled `wasi:*`-importing artefact for the
    /// host-side reject path).
    #[test]
    fn default_deny_linker_built_without_panicking_for_each_grant_subset() {
        let host = WasmHost::new(test_caps()).expect("engine init");
        // Empty grant → only the always-linked io baseline (poll +
        // streams) lands in the linker. Build must succeed.
        let _ = build_linker(host.engine(), WasiCategorySet::empty())
            .expect("empty grant builds linker");
        // Each single grant builds. Multi-grant builds. The point of
        // exercising every subset is to catch a `add_to_linker_get_host`
        // signature mismatch — wasmtime-wasi minor bumps occasionally
        // change the closure trait bounds and the failure would only
        // surface when a consumer actually granted that category.
        for cat in WasiCategory::ALL {
            let _ = build_linker(host.engine(), WasiCategorySet::just(*cat))
                .expect("single-grant linker builds");
        }
        let mut all = WasiCategorySet::empty();
        for cat in WasiCategory::ALL {
            all.insert(*cat);
        }
        let _ = build_linker(host.engine(), all).expect("full-grant linker builds");
    }
}
