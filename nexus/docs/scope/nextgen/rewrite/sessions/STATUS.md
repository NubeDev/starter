# Nexus Rewrite — Build Status Board

> Single source of truth for the orchestration loop. The loop reads this file on every wake.
> Each session updates its own row when it starts, blocks, or finishes.
> All work lands on branch **`nexus-rewrite`** (sequential — one RW at a time, no worktrees).

**Legend:** ⬜ pending · 🔵 in-progress · ✅ done (build+tests green, committed) · ⛔ blocked (see [TODOs.md](./TODOs.md))

## Execution queue (dependency order — DO NOT reorder)

| Order | RW | Title | Status | Started | Finished | Commit | Notes |
|------:|----|-------|:------:|---------|----------|--------|-------|
| 1 | RW-01 | Engine core: native pipeline loop, node traits, registry | ✅ | 2026-06-10 02:20 UTC | 2026-06-10 02:58 UTC | 9757df7c | additive only; ArkFlow stays compiling; 5 core tests green |
| 2 | RW-02 | Port nodes onto core (DataFusion direct) | ✅ | 2026-06-10 02:30 UTC | 2026-06-10 03:25 UTC | 6b25bb7d | behavior parity, same registry names; 40 engine tests green |
| 3 | RW-03 | Cutover: runners on native engine; delete ArkFlow | ✅ | 2026-06-10 02:55 UTC | 2026-06-10 04:35 UTC | 8d679c5b | grep-zero arkflow; vendor/ gone; 207 tests green |
| 4 | RW-04 | Any-DB store: datasource-id sinks, batched writes | ✅ | 2026-06-10 03:25 UTC | 2026-06-10 03:47 UTC | 70a48deb | postgres COPY + file(parquet) writers; audited resolve |
| 5 | RW-05 | Federation: DataFusion across datasources + file kinds | 🔵 | 2026-06-10 03:55 UTC | | | push-down path untouched |
| 6 | RW-06 | nexus-insights: Polars + Rhai sandbox + query stage | ⬜ | | | | migration 18xx; DTO-first |
| 7 | RW-07 | Extension data-plane: sources/sinks/insights contributions | ⬜ | | | | ingest.write host method |
| 8 | RW-08 | Backpressure hardening + soak + flow metrics | ⬜ | | | | BACKPRESSURE.md + make soak |
| 9 | RW-09 | Transport breadth: http_ingest + Zenoh source | ⬜ | | | | human-added 2026-06-10; zenoh feature-gated OFF |

## Loop log (append one line per wake)

<!-- Format: `YYYY-MM-DD HH:MM — <action taken>` -->
2026-06-10 02:20 — RW-01 picked (first pending, no RW in progress); marked 🔵, spawning subagent.
2026-06-10 02:58 — RW-01 done: nexus-engine/src/core/ landed (node traits, registry, config, pipeline, outcome, error); 5 core tests + full engine suite green; ArkFlow still compiles. Marked ✅.
2026-06-10 02:30 — RW-02 picked (first pending, no RW in progress); marked 🔵, spawning subagent.
2026-06-10 03:25 — RW-02 done: native ports of every source/processor/sink onto core traits + native built-ins (memory/generate/http_poll/simulator/json_to_arrow/sql/collector/sse/postgres/drop/stdout) under same registry names; ArkFlow still compiles in parallel; 40 engine tests + workspace test green, clippy clean. Committed 6b25bb7d. Marked ✅. Three RW-01 core deltas (&mut self / commit() / max_batch_rows slice) + sql JSON-UDF confirmation logged to TODOs.md for RW-03.
2026-06-10 02:55 — RW-03 picked (first pending, no RW in progress); marked 🔵, spawning subagent.
2026-06-10 04:35 — RW-03 done: runners (query/live/flow-manager) + registry cut over to core::Pipeline; core §6 deltas landed (Processor::process(&mut self), Source::commit() no-op hook, max_batch_rows zero-copy slice); ArkFlow git deps + [patch] block + vendor/arkflow-plugin/ (35 files) + every arkflow source file deleted; grep -ri arkflow over backend .rs/.toml/.lock + openapi.json = zero; stored-config parity test proves old-engine flows run unchanged; 207 workspace tests green, 0 failed. openapi.json + FE client regenerated (description-only). Marked ✅.
2026-06-10 03:25 — RW-04 picked (first pending, no RW in progress); marked 🔵, spawning subagent.
2026-06-10 03:47 — RW-04 done: `datasource` sink (sink/datasource/: writer trait, postgres COPY writer via sqlx PgCopyIn text format, parquet rotating part-files, batch accumulator flush-on-rows-or-timer, strict identifier guard) registered in native_registry; secret resolution stays in store (resolve_sink_config, audited open_secret) + api (resolve_flow_output) with flows/start.rs as the thin transport seam — engine keeps zero nexus-store dep. Legacy postgres sink configs still build (parity test). e2e docker flow lands rows via COPY; parquet read back via DataFusion. Full engine suite + workspace green, clippy clean. No migration/DTO/codegen needed. Palette-descriptor follow-up logged to TODOs.md (RW-03 registry/ lane). Marked ✅.
2026-06-10 03:55 — RW-05 picked (first pending, no RW in progress); marked 🔵, spawning subagent.
