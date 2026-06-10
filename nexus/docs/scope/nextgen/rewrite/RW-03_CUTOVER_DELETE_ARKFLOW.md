# RW-03 — Cutover: runners on the native engine, ArkFlow deleted

> Verified: 2026-06-10 against master (6b6f16d2). §0: re-grep every file:line below first.
> Depends on RW-01 + RW-02 committed.

## Current state

- `runner/query.rs` ~45-80: builds an ArkFlow StreamConfig (input→pipeline→collector),
  `stream.run(token)`, drains collector, enforces caps → JSON.
- `runner/live.rs` ~33-54: input→pipeline→sse, spawned with token; cancellation when the
  last subscriber drops via `stream_registry.rs` ~80-125 refcounting.
- `flow/manager.rs` ~74-126: named long-lived flows, tracked stats, explicit stop().
- `runner/poll.rs`, `runner/cancel.rs`: supporting pieces — check for ArkFlow types.
- Pins: `backend/Cargo.toml:63-64` (git deps), `:116-121` (vendor patch),
  `backend/vendor/arkflow-plugin/` (the trimmed fork), `registry/mod.rs` ArkFlow init.

## Scope

1. Switch all three runners (and poll/cancel helpers) from ArkFlow StreamConfig/Stream to
   `core::PipelineConfig`/`Pipeline::run`. The JSON config shape, public fn signatures,
   HTTP/SSE wire behavior, caps, and FlowStats are FROZEN — this is an engine swap, not
   a redesign.
2. Replace `registry/mod.rs` ArkFlow registration with the RW-02 native registry; an
   engine `Registry` handle lives wherever the ArkFlow global init lived (follow the
   existing wiring through `nexus-api/src/main.rs`/`state.rs` — append-only there).
3. Delete: `arkflow-core`/`arkflow-plugin` from `backend/Cargo.toml` + the
  `[patch."https://github.com/arkflow-rs/arkflow"]` block + `backend/vendor/arkflow-plugin/`
  + every remaining `use arkflow` / `arkflow_` reference (`grep -ri arkflow` must end at
  zero hits outside docs/).
4. `cargo update` fallout: removing the git dep may shift transitive Arrow/DataFusion
   versions — keep them pinned to what RW-02 chose.

## Acceptance

- `grep -ri arkflow nexus/backend --include='*.rs' --include='*.toml'` → only doc/comment
  hits in scope docs; no code or manifest hits.
- Full engine + api test suites green: collector caps + truncation, SSE seq + Last-Event-ID
  resume, postgres sink flows e2e, flows dry-run route, live stream cancel-on-last-drop.
- `Cargo.lock` no longer contains arkflow; vendor dir gone.
- Flows created under the old engine (stored JSON configs in a dev DB / fixtures) start
  and run under the new engine without migration — add a fixture-based test proving it.
