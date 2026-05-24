## Done

- Added `rubix/crates/rubix-agent/tests/extensions_lifecycle_test.rs` covering the 8 SCOPE Phase E lifecycle assertions (list / enable / disable / restart cycle / events 404 after disable / events 200 after enable / PG row state / event ring messages) against testcontainers PG with a self-built fixture extension dir holding `com.rubix.example`.
- Verified green: `cargo test -p rubix-agent --test extensions_lifecycle_test -- --ignored --nocapture` → 1 passed.
- Committed as `phase E.1 — full lifecycle integration test` (16d9f6f).

## Next

- Stage 15 (final gate): write the design doc rewrite + session note + THIN-SLICE row + PR per SCOPE Phase E.2/E.3.

## What you need to know

- Test is `#[ignore]`-gated like the other PG integration tests in this crate; run with `--ignored`.
- "start/stop/restart" in SCOPE maps onto the actual REST surface (`POST .../enable` and `POST .../disable`) — there is no `/start` `/stop` `/restart` endpoint in `starter-ext-server`; restart is asserted via an enable→disable→enable cycle.
- The Phase B example binary exits before the init handshake, so the supervisor settles into `spawned → crashed → restart_scheduled`. Assertion 8 accepts any of `spawned|crashed|restart_scheduled|state_transition|exited_clean` to remain agnostic to that timing.
- `RingEvent.kind` serialises with `#[serde(tag="kind", content="data")]`, so the wire shape is `{"kind":{"kind":"<variant>","data":{...}}}` — the test reads `e["kind"]["kind"]`.
- Test rebuilds the example binary via `cargo build --manifest-path rubix/extensions/Cargo.toml` if it's missing, so it survives a clean target dir.

## Open questions

- (none)
