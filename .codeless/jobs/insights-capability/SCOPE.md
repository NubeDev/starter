# Scope — insights-capability

The authoritative design lives at
[`/home/user/code/rust/starter/DOCS/Insights/SCOPE.md`](/home/user/code/rust/starter/DOCS/Insights/SCOPE.md).
This brief is the trimmed per-job scope. Where this disagrees with the
source SCOPE, **the source SCOPE wins** — fix this file rather than
diverge.

## Goal

Land the Insights capability end-to-end on the `starter` repo via the
`codeless/insights-capability` branch. After this job, a consumer
composing a starter binary adds **one crate** (`starter-insights`) to
`Cargo.toml`, optionally enables the `insights` feature on a store
crate, adds one or more rule-pack extensions, and gets the full
capability: rule trait, six rule authoring surfaces, windowing,
align + verdict.join, locked Rhai sandbox, AI rule kinds, materialised
verdict log + rollups + derivation cache, four reference rule packs
(iot, energy, hvac, finance), three skill bundles, and the SLO-gated
read paths.

## In scope (the whole SCOPE, in four stages mirroring its Phasing)

- **Phase 1 (stage 1):** crate skeleton, contracts in `starter-spi`
  (D1), `rule.rust` + `verdict.join` node kinds on
  `starter-flow-nodes`, `RuleRegistry` + `QualityFlagRegistry`,
  sqlite-feature persistence (verdict log + tag index only),
  `starter-ext-insights-iot` with three rules and the
  `iot.quality.*` flags. R-ins-9 invariant: rule chaining + error
  routing use existing flow nodes (`branch`, `gate`, `retry`);
  insights ships node bodies only.
- **Phase 2 (stage 2):** `rule.rhai` + locked sandbox (R-ins-4),
  `window.tumble` + `window.slide` (with `tz`), `rule.sql` against the
  host store (D2 Phase 1 shape), backfill with the D3 100k cap +
  `BackfillTruncated` event, verdict rollups (tier 2 materialisation,
  incremental, per-window watermark per D5), `rollup_invalidation`
  table, tag-grouped aggregates (R-ins-8), `confidence_penalty`
  enforcement, `starter-ext-insights-energy`.
- **Phase 3 (stage 3):** `rule.derive` + `align` node (with `NodeId`
  audit) + derivation cache (tier 3) + `StreamingDatasetRows`,
  `rule.ai-check` + `rule.ai-debug` (R-ins-10), three skill bundles
  (`rule-author`, `explain`, `tuner`) under `skills/`, model-family
  pinning + per-`Verdict` exact-model evidence, CI dep-tree gate
  (R-ins-5), auto-tagging, onboarding-backfill cache-warming contract,
  `starter-ext-insights-hvac`.
- **Phase 4 (stage 4):** `starter-ext-insights-finance`, performance
  pass on `verdict.join` and derivation cache against the **D9 SLO
  table** (verdict list 50 ms p95, filtered-by-tag 150 ms,
  rollup timeseries 100 ms / tag-grouped 250 ms, derivation cache
  fetch 50 ms, onboarding cold page 100 ms with `partial-onboarding`
  banner) enforced by a CI smoke over a synthetic 90-day dataset.

## Out of scope

- **A general-purpose stream processor.** Insights windows are bounded
  per the SCOPE non-goals.
- **Dashboarding UI.** Operator UI panels in `starter-ui-*` are
  consumer-owned, not part of this capability (SCOPE non-goal).
- **A replacement for monitoring systems.** Insights *complements*
  Prometheus/OpenSearch/Datadog via existing scrape/notify tools.
- **A workflow language.** Pipelines are flows; this job adds node
  kinds, not a DSL.
- **A Windmill clone.** Lifted concepts only — scripts-as-composables
  + job history (the engine already has these). No parallel worker
  pool, no parallel auth, no polyglot runtime, no workspaces concept.
- **A second store crate, a second migration runner, a second
  `starter-insights-spi` crate.** R-ins-1 binds — one root crate.
- **A second orchestrator inside insights.** R-ins-9 binds — chaining,
  retry, branching, error routing all stay on the flow engine.
- **A second LLM seam.** R-ins-5 + agent R2 bind — every LLM call
  routes through `AiRunner`; the dep-tree gate enforces this on
  rule packs and on insights itself.
- **Streaming late-arriving per-row correction.** D5 covers
  per-window retroactive only; per-row belongs in a stream processor
  upstream of insights.
- **Unbounded backfill.** D3 hard-caps at 100k rows per invocation;
  unbounded is a follow-up `backfill.batched` node kind, not this
  job.
- **Per-row arrow-buffered datasets.** `Dataset::rows` stays
  `Arc<dyn DatasetRows>` with `VecDatasetRows` in `starter-spi` and
  `StreamingDatasetRows` in `starter-insights` (D1). Heavy variants
  are a future trait-object hop, not a `starter-spi` widening.
- **Custom node kinds shipped by packs in Phase 1–3.** D7 allows it
  but the four reference packs in this job all contribute *rules*,
  not nodes. If a pack maintainer raises a custom-node need during
  the job, write it up as a follow-up; do not silently expand scope.

## Constraints

- **R-ins-1** — one root crate (`starter-insights`). No
  `starter-insights-spi`, no `-rhai`, no `-sql`, no `-store`. Modules
  are internal.
- **R-ins-2** — rules are reusable by reference; the registry is the
  seam.
- **R-ins-5** — every LLM call routes through `AiRunner`; CI dep-tree
  gate refuses any pack that pulls a provider SDK directly.
- **R-ins-7** — derivation rules and assertion rules share **one**
  `Rule` trait, two output shapes (`Verdict` vs `Dataset`); no
  parallel "transform" concept.
- **R-ins-8** — `Tags` are first-class metadata, merged from
  rule + pipeline-node onto every `Verdict`/`Dataset`, indexed.
- **R-ins-9** — chaining + branching + retry + error routing are
  flow-engine concerns; insights ships node bodies only.
- **R-ins-10** — AI is a first-class rule kind, not a meta helper.
- **R-ins-11** — quality flags are extensible by pack via namespaced
  `QualityFlagId`s.
- **Flow engine R1/R3/R8/R9** apply transitively.
- **Agent R2 + R4** apply to the three skill bundles unchanged.
- **starter root R0** — no monolith re-imports, no Windmill surface
  area drift.

## Deliverables (what "done" looks like)

1. `codeless/insights-capability` branch with one commit per stage,
   pushed via mani; commits 1, 3, 5, 7 land the phase deliverables,
   commits 2, 4, 6 are the REVIEW handovers.
2. `cargo test --workspace` green on the `starter` repo at every
   stage boundary.
3. `cargo clippy --workspace --all-targets -- -D warnings` green at
   every stage boundary.
4. `cargo fmt --check` green at every stage boundary.
5. Four reference smokes pass, reproducing the SCOPE §"Use-case fit"
   rows in `starter/DOCS/Insights/SCOPE.md`:
   - **IoT row** (stage 1 close)
   - **Energy row** (stage 2 close)
   - **HVAC row + bills-reconciliation row** (stage 3 close)
   - **Finance row** + **SLO smoke** over the 90-day synthetic
     dataset (stage 4 close)
6. CI dep-tree gate (R-ins-5) passes on a synthetic violator
   (added in stage 3) and fails as expected.
7. The `insights` feature on `starter-store-sqlite` and
   `starter-store-postgres` compiles independently of any extension
   pack.
8. `starter/DOCS/Insights/SCOPE.md` "Phasing" section: phases 1–4
   flip `[ ] → [x]` (or equivalent) in the same edits that land
   each phase.

## Open questions (resolve in stage 1, before any production code)

The SCOPE's **D-list** is already resolved upstream (D1–D9). This
job inherits those decisions. Three job-specific open questions to
resolve here:

1. **Scope realism — is this honestly one job?** The source SCOPE
   describes 4 phases, ~10 new node kinds, 4 extension packs, an
   `AiRunner` integration, a CI dep-tree gate, a locked Rhai
   sandbox, materialisation across three tiers, and an SLO suite —
   that is several weeks of work, not one cost-capped run.
   Bias: keep as one job per the user's request, but **explicitly
   raise the cap mismatch at the start of stage 1** and surface
   any phase as a separable follow-up if the runner's cost cap is
   hit. Do **not** silently land partial work; mark the stage `[!]`
   per CLAUDE.md R4 and stop.
2. **Where do the `rule.*` node kinds physically live?** The SCOPE
   says they are contributed to `starter-flow-nodes`'s
   `NodeKindRegistry` but implementations live in `starter-insights`
   and are registered by the host at boot (lines 162–169). Confirm
   in stage 1 against the current `starter-flow-nodes` layout that
   this registration shape works without circular crate deps; if it
   doesn't, the resolution is a `register_insights_nodes()` function
   in `starter-insights` that the host binary calls — **not** a new
   crate, **not** an inversion of the dep arrow.
3. **`starter-spi` contract churn.** D1 places `Rule`, `Verdict`,
   `Coverage`, `Dataset`, `RuleId`, `RuleSchema`, `RuleError`,
   `Severity`, `QualityFlag`, `QualityFlagId`, `Tags`, `Window`,
   `TimeZoneId` in `starter-spi`. Confirm at stage-1 start how many
   downstream crates today depend on `starter-spi` and whether any
   of them will be broken by additions; bias is yes-they're-additive
   because everything new is a new symbol, but a wholesale
   pub-use sweep needs an audit before landing.

Record each chosen answer + a one-line *why* in this file during
stage 1; no production code in stage 1 before all three are
resolved.

## References

- Source SCOPE (authoritative):
  [/home/user/code/rust/starter/DOCS/Insights/SCOPE.md](/home/user/code/rust/starter/DOCS/Insights/SCOPE.md)
- Flow engine SCOPE: `starter/DOCS/flow/scope/SCOPE.md`
- Agent SCOPE: `starter/DOCS/agent/SCOPE.md`
- Extensions SCOPE: `starter/DOCS/extensions/scope/SCOPE.md`
- Agent rules: `starter/CLAUDE.md` (if present at root) and the
  workspace-level `codeless-workspace/CLAUDE.md` for mani / commit
  discipline.
