# REVIEW gate 2 (slice D) — Layer-1 verdict: PASS

Scope: R1 crate dep direction, R2 single transport, R4/R5 trust
boundary, wire-formats untouched. Functional spec coverage is
out of scope for this gate (deferred to a later ramp step).

## Layer-1

- R1: starter-warehouse → {starter-store-postgres[dimensions],
  starter-store-warehouse, starter-tags, starter-flow-spi,
  starter-flow-nodes, starter-server}. No back-edges. ✓
- R2: REST handlers in `src/rest/mod.rs` forward into
  `WarehouseRuntime`. Nodes delegate to the same runtime
  methods. Single body per kind. ✓
- R4/R5: W12 ext re-quarantine inside one tx in `mart_define`;
  W14 promoted-column filter check precedes any CH query;
  W7 honoured by `tap_write` (no payload refusal); W8 ingest
  goes through `starter-store-warehouse::store::*`. ✓
- Wire formats: no edits to slice A, slice B PG migrations,
  slice C CH migrations, or starter-store-sqlite. ✓

## Build + tests

- `cargo build -p starter-warehouse --features warehouse` — green
- `cargo test -p starter-warehouse --features warehouse` — 13/13
  unit tests green; with_stack.rs requires docker (not run)

## Functional gaps (flag for ramp step, not Layer-1 fail)

- `mart_read` returns empty `rows` — actual CH SELECT not wired.
- No transcript for dim_freshness stale / failed_refresh states.
- No transcript for W15 GC against a 90-day-old quarantined row
  (dry-run + real-run).
- No transcript for W16 read-after-write ≤1.5s bound.
- No transcript for cleaner sandbox-freeze end-to-end flow.
