# RW-02 — Port nodes onto the core engine (DataFusion direct)

> Verified: 2026-06-10 against master (6b6f16d2). §0: re-grep every file:line below first.
> Depends on RW-01 (core traits committed — read `nexus-engine/src/core/node.rs` first).

## Current state

All of these are nexus-authored but implement **ArkFlow traits** (`Input`/`Output`/
`OutputBuilder` etc.):

- Sinks: `nexus-engine/src/sink/collector.rs` ~34-54 (bounded accumulator + `cap.rs`
  CapState), `sink/sse.rs` ~38-48 (broadcast + seq), `sink/postgres.rs` ~55-74
  (Arrow→JSON→INSERT), plus `broadcast_store.rs` / `store.rs`.
- Sources: `source/http_poll.rs` ~51-73, `source/simulator.rs` ~73-81 (+ `sim.rs`).
- Processors come from ArkFlow's trimmed plugin: `sql` (DataFusion over batches) and
  `json_to_arrow`; `arrow_json.rs` ~24-50 holds the nexus Arrow→JSON bridge.
- ArkFlow built-ins still referenced: `memory`, `generate` inputs (tests/dry-run),
  `drop`, `stdout` outputs.

## Scope

1. Re-home every nexus source/sink onto the RW-01 traits. This is mostly mechanical:
   same struct, same config parsing, `read()/write()/close()` instead of ArkFlow's
   trait surface. Do NOT change behavior — caps, SSE seq numbers, Last-Event-ID resume,
   postgres batching all stay bit-identical.
2. Write native replacements for the ArkFlow built-ins we use:
   `processor/sql.rs` — DataFusion `SessionContext`, register incoming batch as a table
   (same table name convention the current sql processor exposes — grep stored flow
   configs/tests for it), run the configured query, emit result batches.
   `processor/json_to_arrow.rs` — `arrow-json` Decoder. Schema stability per roadmap §6:
   today's per-batch inference is ArkFlow's accident, not a contract — implement
   infer-on-first-batch-then-coerce (incoercible batch = source error), with an optional
   declared schema in flow config taking precedence. Existing tests' expectations still
   hold for the single-batch cases they cover.
   `source/memory.rs`, `source/generate.rs`, `sink/drop.rs`, `sink/stdout.rs` — trivial.
3. Register everything in a `core::Registry` factory fn (`registry/native.rs` or similar)
   under the SAME string names the JSON configs use today — stored tenant flow configs
   must keep working unchanged.
4. Add `datafusion` + `arrow-json` as direct deps of `nexus-engine` (workspace already
   resolves them transitively — pin the same major versions ArkFlow pulls today to keep
   one Arrow in the tree).

## Non-goals

Switching the runners over (RW-03) — old ArkFlow registrations stay alive in parallel;
both registries coexist for one workstream.

## Acceptance

- `cargo test -p nexus-engine` green; every existing sink/source test passes against the
  ported impls (port the tests with the code — assertions unchanged).
- New tests: sql processor parity (same query, same input batch → same output as a
  recorded fixture from the current engine), json_to_arrow round-trip,
  memory/generate finite-end semantics.
- No behavioral diff in SSE seq/resume or collector truncation flags (assert in tests).
