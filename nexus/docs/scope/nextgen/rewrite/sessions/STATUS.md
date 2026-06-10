# Nexus Rewrite — Build Status Board

> Single source of truth for the orchestration loop. The loop reads this file on every wake.
> Each session updates its own row when it starts, blocks, or finishes.
> All work lands on branch **`nexus-rewrite`** (sequential — one RW at a time, no worktrees).

**Legend:** ⬜ pending · 🔵 in-progress · ✅ done (build+tests green, committed) · ⛔ blocked (see [TODOs.md](./TODOs.md))

## Execution queue (dependency order — DO NOT reorder)

| Order | RW | Title | Status | Started | Finished | Commit | Notes |
|------:|----|-------|:------:|---------|----------|--------|-------|
| 1 | RW-01 | Engine core: native pipeline loop, node traits, registry | ⬜ | | | | additive only; ArkFlow stays compiling |
| 2 | RW-02 | Port nodes onto core (DataFusion direct) | ⬜ | | | | behavior parity, same registry names |
| 3 | RW-03 | Cutover: runners on native engine; delete ArkFlow | ⬜ | | | | grep-zero arkflow; vendor/ gone |
| 4 | RW-04 | Any-DB store: datasource-id sinks, batched writes | ⬜ | | | | postgres + file(parquet) writers |
| 5 | RW-05 | Federation: DataFusion across datasources + file kinds | ⬜ | | | | push-down path untouched |
| 6 | RW-06 | nexus-insights: Polars + Rhai sandbox + query stage | ⬜ | | | | migration 18xx; DTO-first |
| 7 | RW-07 | Extension data-plane: sources/sinks/insights contributions | ⬜ | | | | ingest.write host method |
| 8 | RW-08 | Backpressure hardening + soak + flow metrics | ⬜ | | | | BACKPRESSURE.md + make soak |

## Loop log (append one line per wake)

<!-- Format: `YYYY-MM-DD HH:MM — <action taken>` -->
