## Done

- rhai sandbox (R-ins-4) locked in `crates/starter-insights/src/rhai_sandbox.rs`; eval/import denied, ops/string/array/map/depth caps wired, per-rule budget override surfaced
- `rule.rhai` node (`nodes/rule_rhai.rs`) with D4 anonymous RuleId via blake3 of the script body
- `window.tumble` + `window.slide` (`nodes/windowing.rs`) with mandatory IANA tz, anchored on local midnight, emit JSON Dataset projections
- `rule.sql` (`nodes/rule_sql.rs`, sqlite-gated) — D2 Phase 1 shape against the host primary SQLite pool
- D3 backfill cap + `BackfillEvent::{Completed,Truncated}` + `partial-onboarding` flag on truncated rows (`backfill.rs`)
- `penalty::apply_derivation_penalty` engine helper; `RuleRegistry::register` rejects `confidence_penalty > 1.0` (R-ins-6)
- Phase 2 SQLite migration `0002_rollups.sql` (verdict_rollup, rollup_checkpoint, rollup_invalidation) with `IFNULL`-keyed composite unique index
- `rollups::RollupEngine` incremental ticks + tag-grouped aggregates (R-ins-8) + D5 enqueue/drain invalidation + stale_since_ms marker
- `starter-spi::insights::RuleSchema` gains `confidence_penalty`, `retroactive`, `idempotent`, `persist`, `max_operations`; `Dataset::from_parts` builder; new `retroactive_correction_flag` + `partial_onboarding_flag` helpers + id constants
- New `starter-ext-insights-energy` pack (5 rules + 2 `energy.quality.*` flags), wired into the `starter-extensions` workspace
- Energy smoke (`tests/energy_smoke.rs`) reproducing the Energy row + Rhai custom rule + tz-aware window.slide + penalty chain + registry rejection + D5 retroactive path + incremental tag-grouped rollups
- IoT Phase 1 smoke still green
- Committed as "stage 2 — Phase 2 (rhai + sql + windowing + rollups + energy pack)"

## Next

- Stage 3 / Phase 3: `rule.derive` + `align` node + derivation cache (tier 3) + `StreamingDatasetRows` + `rule.ai-check` + `rule.ai-debug` + the three skill bundles; reproduces HVAC + bills-reconciliation rows

## What you need to know

- `rhai 1.x` with `default-features=false, features=["std","sync","no_optimize"]` added at the workspace level; `chrono-tz 0.9` likewise
- The energy pack is referenced by `starter-insights`'s `dev-dependencies` via path (same pattern as iot); it lives in the sibling `starter-extensions` workspace and is excluded from the parent workspace
- SQLite forbids expressions in PRIMARY KEY / UNIQUE table constraints — composite uniqueness for `verdict_rollup` lives on an explicit `IFNULL(...)`-keyed `CREATE UNIQUE INDEX` and the upsert `ON CONFLICT` matches that target list verbatim
- The `Severity` enum is `#[non_exhaustive]`; the rollup severity-bucket match needs a `_` arm (treated as `Info`)
- `Dataset` is `#[non_exhaustive]` so external crates must use `Dataset::from_parts` (added in this stage) — the iot pack happens not to construct Datasets so it was untouched
- Phase 3 will need `align` (a `Frame` slot type) — `verdict.join` already consumes JSON-serialised verdicts so an `align` node can emit a JSON tuple of Dataset projections without changing the SlotValue enum

## Open questions

- (none)
