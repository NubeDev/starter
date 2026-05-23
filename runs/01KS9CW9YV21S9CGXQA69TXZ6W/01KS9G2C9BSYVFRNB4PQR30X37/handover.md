## Done

- Inspected slice-D diff (commits 7331cc9 + 723e812): warehouse crate, REST surface, runtime, DDL gen, GC, audit, nodes.
- Verified Layer-1 invariants:
- R1: starter-warehouse depends only on stores + flow SPI + server; no back-edges.
- R2: every REST handler forwards into a single `WarehouseRuntime` method; nodes delegate to the same runtime.
- R4/R5: W7/W8/W12/W14 trust-boundary code paths in place (ext-hash re-quarantine inside one tx, promoted-column filter validation before CH query, async_insert routed through store crate, tap_write never refuses payload).
- Wire formats: slice A, slice B Postgres migrations, slice C CH migrations, and starter-store-sqlite untouched.
- `cargo build -p starter-warehouse --features warehouse` — green.
- `cargo test -p starter-warehouse --features warehouse` — 13/13 unit tests green.
- Wrote DOCS/Warehouse/reviews/stage6-gate2.md and committed as stage 6 PASS.

## Next

- (none) — next session is stage 7 (slice E: flow-agent example smoke + iot-anomaly-detector port + final sweep), but a ramp step should first close the functional gaps listed below.

## What you need to know

- Verdict line: `PASS: <one-sentence reason>` printed in the reply.
- Gate scope per harness instruction was narrow (Layer-1 only, no patch proposals). Several spec-mandated transcripts demanded by the stage prompt are missing — they are deliverable gaps, not Layer-1 failures, and were flagged in the review note instead of failing the gate.
- Functional gaps to address before/during slice E:
- `WarehouseRuntime::mart_read` returns `rows: Vec::new()` — actual ClickHouse SELECT against the generated mart table is not wired.
- No integration test for dim_freshness `stale_within_bound` / `stale_beyond_bound` / `failed_refresh` transitions (only a `populated` smoke).
- No integration test for W15 GC dry-run + real-run against a 90-day-old quarantined row.
- No integration test asserting W16 ≤1.5s read-after-write via `/api/warehouse/status.ingest.async_insert_oldest_age_ms = 0` polling.
- No integration test for the cleaner sandbox-freeze end-to-end flow (only the standalone `dim::sandboxes::freeze` is exercised in the RF-4 test).
- with_stack.rs tests require docker testcontainers; they were not executed in this sandbox.
- `mart_read` in `src/rest/mod.rs` caps `max_buckets` at 100_000 instead of the SCOPE-documented 20_000 default — worth flagging in the ramp step.

## Open questions

- Should the ramp step land inside slice E or be its own stage? The stage prompt forbids advancing without explicit approval, so a human decision is required before slice E starts.
