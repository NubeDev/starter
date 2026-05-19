## Done

- Created `starter-extensions/crates/starter-ext-wasm/` with wasmtime 30.0.2-based `WasmHost` (component-model + fuel + epoch interruption), `Caps`/`WasiCategorySet` per-call resource + grant shape, `MemoryLimiter` over `StoreLimits`, stateless `InstanceState` per call, `WIT_PACKAGE` embedded constants
- Wrote `wit/starter-extension.wit` (package `starter:extension@0.1.0`) with `stream-event`/`stream-end`/`stream-error`/`stream-cancel` host imports reserved, `kv` commented for v0.2, mandatory `guest.dispatch-tool` export, `world extension`
- Default-deny linker: always wires `wasi:io/poll` + `wasi:io/streams` baseline (resource plumbing only, no capability); per-call adds `wasi:clocks/wall-clock`+`monotonic-clock` for `WallClock`, `wasi:filesystem/types`+`preopens` for `Fs`; `HttpOut` logs a `tracing::warn!` placeholder since `wasmtime-wasi-http` is intentionally not pulled in
- Caps: `max_fuel` → `Store::set_fuel`, `max_memory_bytes` → `Store::limiter`, `wall_clock_deadline` → `Store::set_epoch_deadline` (translated through `epoch_ticks_for` against a 100Hz `epoch_ticker` tokio helper the consumer spawns)
- `Caps::new` rejects zero fuel / zero memory / zero duration so silent disablement is impossible
- Updated `starter-ext-sdk/src/wasm.rs` (was a Stage-4 stub returning `Err`) to provide `run_wasm_main` + stub `CtxInner` mirroring the process flavour's stub shape
- Added `register_wasm_main!` macro in `starter-ext-sdk/src/lib.rs` (gated by `wasm` feature) emitting `pub fn dispatch(tool_id, params) -> Result<Value>`
- Fixed the pre-existing `flavour_marker_symbol_has_expected_value_for_builtin_feature` test to cfg-gate per active feature so it passes under `--features wasm` / `--features process` too
- Added `examples/hello-wasm/` (lib + cdylib): same trait impl as hello-builtin/hello-process byte-for-byte; only the `register_wasm_main!` macro and `runtime.kind: wasm` differ
- Wired workspace `Cargo.toml`: added `crates/starter-ext-wasm` + `examples/hello-wasm` to members, added `starter-ext-wasm` workspace dependency
- Tests: 14 unit tests in `starter-ext-wasm` all pass (`cargo test -p starter-ext-wasm`), and the SDK passes under each individual flavour feature

## Next

- (none) — Stage 11 finished. Adapter Phase 5+ stages pick up next (REST/CLI/gRPC/UI adapters, then the cross-cutting periodic-worker contribution surface)

## What you need to know

- **`cargo check --workspace` is broken by design** — the SDK's `__STARTER_EXT_FLAVOUR_MARKER` `#[no_mangle]` static is the R1 linker-error trick, so unifying `builtin` + `process` + `wasm` features (which workspace-wide cargo does) trips the duplicate-symbol diagnostic. This pre-existed Stage 11 (the workspace had hello-builtin + hello-process). Use `cargo check -p <crate>` per crate instead — every package builds cleanly in isolation
- The typed `dispatch-tool` call body returns `WasmCallOutcome::NotImplemented` today — `wit_bindgen` on the guest side + `wasmtime::component::bindgen!` on the host side were not added (large compile cost, additive when needed). The kernel shape is complete; only the typed JSON transit is deferred. Adapter code can match the full `WasmCallOutcome` enum non-exhaustively today
- `HttpOut` grant currently logs a `tracing::warn!` and leaves `wasi:http` unlinked — `wasmtime-wasi-http` is a separate crate intentionally not added in Stage 11 to keep the dependency footprint small. A future minor adds it as a one-line additive change when an extension actually needs HTTP out
- Wasmtime is pinned to `=30.0.2`; the per-interface `add_to_linker_get_host` API the host calls into is private-shape across minor bumps, so pinning is load-bearing
- `epoch_ticker` is NOT spawned by the host — the consumer's runtime spawns it. Without it, the `wall_clock_deadline` cap never fires (documented behaviour, not a silent default)

## Open questions

- (none)
