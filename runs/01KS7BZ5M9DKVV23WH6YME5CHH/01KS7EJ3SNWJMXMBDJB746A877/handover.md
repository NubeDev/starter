## Done

- Durable D5 fix: `retroactive::attach_retroactive_flag` + `MutationWatermarks` engine seam; rollup drain re-aggregates invalidated windows instead of just deleting the queue (uses the rollup-config tz, not per-verdict tz, so buckets line up).
- Durable R-ins-4 fix: `tests/fixtures/rhai_known_bad/` ships 8 attack-category scripts + `tests/rhai_sandbox_known_bad.rs` smoke that picks them up automatically.
- Phase 3 nodes: `rule.derive`, `align`, `rule.ai-check`, `rule.ai-debug` (the AI variants go through thin `AiJudge`/`AiDebugger` seams so `starter-insights` stays provider-SDK-free).
- Phase 3 plumbing: `StreamingDatasetRows`, `DerivationCache` (migration 0003), `ModelFamily` + auto-tagging + exact-model evidence, `onboarding::run_onboarding_backfill`.
- Skill bundles at `skills/starter.insights.{rule-author,explain,tuner}/SKILL.md`.
- `starter-ext-insights-hvac` (pmv-comfort + setpoint-drift + short-cycle) registered in the extensions workspace.
- CI dep-tree gate at `scripts/insights_dep_gate.sh` — runs clean.
- `tests/phase3_smoke.rs` reproduces the HVAC row AND the bills-reconciliation row end-to-end; full `cargo test -p starter-insights --features sqlite` passes (37+ tests).

## Next

- Stage 4 — Phase 4: finance pack, perf pass on verdict.join + derivation cache, frontend UI panel (out-of-scope per SCOPE).

## What you need to know

- The AI rule kinds depend on a thin `AiJudge` / `AiDebugger` trait pair (in `crates/starter-insights/src/nodes/rule_ai_*.rs`). The host wires the concrete `AiRunner`-backed impls; the smoke uses stubs. This is what keeps the R-ins-5 CI dep-tree gate green.
- `align` emits a JSON `frame` slot value (not a public registry concept) with the D8 NodeId audit identity baked into the payload as `"node_id"`. The downstream `rule.derive` reads source rows from `frame["sources"]["<src>"]["rows"]`.
- `DerivationCache` lives behind the `sqlite` feature; migration `0003_derivation_cache.sql` is picked up by the existing `INSIGHTS_MIGRATION_SOURCE`.
- `RollupEngine::drain_invalidations` now takes a `tz` arg — the rollup-config tz, not the per-verdict tz. Bucket math has to match the original fold.

## Open questions

- The bills-reconciliation smoke uses a stand-in `Verdict::warn` for the custom Rhai tariff rule rather than driving `rule.rhai` through to compose the verdict; the rhai surface is covered separately in `energy_smoke.rs`. Acceptable per the smoke's intent (covering Phase 3 surfaces); flag if Stage 4 review wants the rhai inline too.
- The `AiJudge`/`AiDebugger` seam intentionally does not implement `starter_spi::ai::AiRunner` directly — a follow-up should ship the host-side adapter that wraps an `Arc<dyn AiRunner>` and implements `AiJudge`. Not in scope for this stage.
