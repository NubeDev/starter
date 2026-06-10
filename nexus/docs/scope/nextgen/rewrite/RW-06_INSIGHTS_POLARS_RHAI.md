# RW-06 — nexus-insights: Polars engine + Rhai sandbox

> Verified: 2026-06-10 against master (6b6f16d2). §0: re-grep every file:line below first.
> Depends on RW-03. Parallel-safe with RW-04/05 except the 🔶 query-route append — land
> after RW-05's dispatch seam if both are in flight (queue order already ensures this).

## Why

Dashboard insights need more than SQL (rolling windows with custom logic, anomaly scoring,
reshaping). The store does the heavy lifting; this is the post-query compute stage.
Design rule: **the script orchestrates, Polars computes** — user code never loops over
rows; it composes vetted vectorized primitives. That keeps it fast AND sandboxable.

## Scope

1. New crate `crates/nexus-insights` (workspace member append):
   - `engine.rs` — Arrow `RecordBatch`es ↔ Polars `DataFrame` (polars feature-trimmed:
     lazy, rolling_window, temporal; no io features needed — data arrives as Arrow).
   - `sandbox.rs` — Rhai `Engine` factory: `set_max_operations`, `set_max_call_levels`,
     `set_max_string_size`/`array_size`, wall-clock timeout via `on_progress` + deadline;
     NO file/network/eval/module APIs registered. One engine per execution (cheap), no
     cross-tenant state.
   - `api.rs` — the curated surface registered into Rhai, first set:
     `select/rename/filter_gt/filter_lt/filter_eq`, `rolling_mean/min/max/sum(col, window)`,
     `zscore(col)`, `resample(time_col, every, aggs)`, `lag/diff/pct_change(col)`,
     `fill_null(strategy)`, `head/tail/sort`, `anomalies(col, z_threshold)` → flag column,
     `describe()`. Each is a thin Polars expression call. Return values are DataFrame
     handles (Rhai custom type), so scripts chain: `df.resample(...).zscore("kw").anomalies("kw", 3.0)`.
   - `run_insight(script, df, params) -> Result<DataFrame>` per roadmap §6, with
     structured errors (compile vs runtime vs limit-exceeded) safe to show tenants.
2. Query-path integration (🔶 small appends): optional `insight` field on the query
   request — `{ "script": "...", "params": {...} }` or a stored insight reference —
   applied to the result batches before the collector/SSE serialization. Caps still apply
   AFTER the insight (it can aggregate down, never explode past caps). DTO-first.
3. Stored insights: `18xx` migration — tenant-scoped `insights` table (id, name, script,
   params_schema, RLS like dashboards) + CRUD routes, mirroring an existing small CRUD
   vertical (folders is a good template). Panels reference an insight id.
4. Tests: every api.rs primitive (golden frames); sandbox kill-switch tests — infinite
   loop trips max_operations, huge string trips size cap, deadline fires; a realistic
   script over simulator data; tenant error surfaces (no panics across the FFI-ish edge).

## Non-goals

UI editor (follow-up WS), extension-contributed insights (RW-07), Python anything
(forbidden by roadmap §8), per-row Rhai callbacks into Polars (perf + sandbox hazard).

## Acceptance

- `cargo test -p nexus-insights` green incl. sandbox-limit tests.
- `POST /query` with an inline insight script transforms results end-to-end (e2e test).
- Insight CRUD + RLS isolation tests green; openapi + codegen committed.
- A pathological script (loop forever / 10^9-row explode attempt) returns a clean
  limit error within the configured timeout — demonstrated in a test.
