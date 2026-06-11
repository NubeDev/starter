# RW-01 — Engine core: native pipeline loop, node traits, registry

> Verified: 2026-06-10 against master (6b6f16d2). §0: re-grep every file:line below first.

## Current state

- ArkFlow supplies the execution loop: `StreamConfig::build()` → `Stream` →
  `stream.run(token)`, consumed by all three runners
  (`nexus-engine/src/runner/query.rs` ~45-80, `runner/live.rs` ~33-54,
  `flow/manager.rs` ~74-126).
- ArkFlow's registry maps config names to builders; nexus registers customs in
  `nexus-engine/src/registry/mod.rs` ~20-36 (`register_input_builder` /
  `register_output_builder` style).
- Data unit is an Arrow `RecordBatch` (ArkFlow `MessageBatch` wraps it).
- Pinned at git rev `b8f82b3` solely for `Stream::run(&mut self, CancellationToken)`.

## Scope

Build `nexus-engine/src/core/` — a self-contained pipeline engine with **zero ArkFlow
imports**, implementing the §6 contracts in the roadmap:

1. `core/node.rs` — `Source` / `Processor` / `Sink` async traits over `RecordBatch`
   (exact signatures: roadmap §6 — note `Processor::process` takes `&mut self`, so
   stateful processors need no interior mutability). Builders take `serde_json::Value`
   config.
2. `core/registry.rs` — `Registry { sources, processors, sinks: HashMap<String, Builder> }`
   with `register_*` + `build_*` fns. No global state; an instance lives on AppState later.
3. `core/pipeline.rs` — `PipelineConfig { input, pipeline: Vec<…>, output }` parsed from
   the SAME JSON shape today's flows/queries use (see any stored flow config / the
   StreamConfig construction in the runners — and grep stored configs/fixtures to confirm
   no flow ever fans out to multiple outputs before freezing the single-output shape),
   and `Pipeline::run(token)`:
   - source task → `tokio::mpsc::channel` (bounded, capacity from config, default 64)
   - `max_batch_rows` enforcement (roadmap §6): oversized batches from source or processor
     output are sliced (`RecordBatch::slice`, zero-copy) before entering the channel —
     bounded-in-batches is only backpressure if batches are bounded in size
   - processor chain applied per batch
   - sink writes; `close()` on end
   - `tokio::select!` on `token.cancelled()` everywhere; cancellation = stop reading,
     drain channel, close sinks, return `Ok(Cancelled)` distinguishable from `Completed`.
4. `core/error.rs` — engine error enum (build / source / processor / sink / cancelled).
5. Unit tests: finite source runs to completion + sink sees all batches in order;
   cancellation mid-stream closes the sink exactly once; bounded channel blocks a fast
   source against a slow sink (backpressure proof — use a sink with a `Notify` gate);
   registry rejects unknown names with a useful error.

## Non-goals

Porting real nodes (RW-02), touching the runners (RW-03), ArkFlow deletion (RW-03).
The crate must keep compiling WITH ArkFlow still present — `core/` is purely additive.

## Acceptance

- `cargo test -p nexus-engine` green, new core tests included.
- No `arkflow` import anywhere under `src/core/`.
- `Pipeline::run` semantics documented on the type (finite end, cancel, error paths).
- Trait/contract docs match roadmap §6 verbatim (they freeze when RW-02 starts).
