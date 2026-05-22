## Done

- Shipped `starter-extensions/crates/starter-ext-insights-finance` with three assertion rules (`finance.tx.z-score@1`, `finance.tx.isolation-forest-light@1`, `finance.tx.duplicate@1`) + two quality flags (`finance.quality.duplicate-timestamp@1`, `finance.quality.fx-rate-stale@1`). All finance verdicts auto-tagged `domain:finance` + `pii`.
- Wired the new pack into `starter-extensions/Cargo.toml` and into `starter-insights`'s dev-deps for the smoke.
- Added `crates/starter-insights/tests/finance_smoke.rs` (8 tests) — registers via the same `RuleRegistry`/`QualityFlagRegistry` seam IoT/Energy/HVAC use; covers happy/critical paths for all three rules plus seed-determinism for isolation-forest-light.
- Added `crates/starter-insights/tests/d9_perf_smoke.rs` — seeds a synthetic 90-day dataset (3 rules × 24h × 90d = 6480 verdicts), rolls up day buckets (ungrouped + `domain`-tag-grouped), populates 90 derivation-cache windows, then samples 50× per read-path and asserts p95 against the SCOPE D9 table (50/150/100/250/50 ms). Gated on `--features sqlite`.
- Exposed `VerdictStore::list_recent_by_rule` / `list_recent_by_tag` and `RollupEngine::read_timeseries_ungrouped` / `read_timeseries_grouped` (+ `RollupBucket` struct) — these are the D9 read-path contracts the SCOPE says must be regression-testable.
- `cargo test -p starter-insights --features sqlite` → 18 + 1 + 8 + 8 + 4 + 9 + 2 = all green; observed p95s on the worktree: `verdict-list=2 tag-list=8 rollup-ts=0 rollup-tag=0 cache=0` ms.
- Committed as `347ce6f` on `codeless/insights-capability` with the stage-4 title.

## Next

- (none) — this was the last stage.

## What you need to know

- No operator UI was shipped here (out of scope per SCOPE "Non-goals"; SCOPE line 1474–1477 calls it consumer-owned).
- The full `cargo build` over the extensions workspace fails with a pre-existing `__STARTER_EXT_FLAVOUR_MARKER` symbol collision in `starter-ext-sdk/src/lib.rs:84` — confirmed pre-existing by stashing my diff and rebuilding from `d8935be`; nothing I changed touched that crate. Individual finance/iot/energy/hvac packs build fine on their own.
- The perf smoke runs at default-feature (debug) profile and still passes with two-orders-of-magnitude headroom on this worker. Per D9 rule 2, if CI ever regresses, the fix is to tighten the query/index/rollup schema — not the budget.
- Finance rules deliberately do not depend on `starter-insights`; per D1 every rule pack depends on `starter-spi` only.

## Open questions

- (none)
