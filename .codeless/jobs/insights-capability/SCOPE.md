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

## Open questions — RESOLVED (2026-05-22 by ap@nube-io.com before start)

The SCOPE's **D-list** is resolved upstream (D1–D9). The three
job-specific open questions are resolved below with the evidence
that supports each decision. The runner does **not** need to
re-litigate these in stage 1; it executes against them.

### Q1 — Scope realism: is this honestly one job?

**Answer: No, it is not one job at the cap originally set, and the
job is restructured to acknowledge that.**

The source SCOPE describes 4 phases: ~10 new node kinds, 4
extension packs, an `AiRunner` integration, a CI dep-tree gate, a
locked Rhai sandbox, three-tier materialisation, and an SLO suite.
That is several weeks of focused work, not a single capped run.
Pretending otherwise would land partial work under a `[!]` halt
midway and waste the runner's budget on context that the next job
re-pays for.

**Decision.**

1. **Phase 1 is the only phase this job commits to.** Stages 1–2
   (Phase 1 implementation + Phase 1 REVIEW) are the deliverable.
   The remaining stages (3–7) stay in `template.yaml` as the
   **declared follow-up plan**, marked clearly as out-of-cap for
   this run.
2. The runner **must stop and request a new job** at the Phase 1
   REVIEW gate. Phase 2 onward each become their own job
   (`insights-capability-phase-2`, `-phase-3`, `-phase-4`), each
   submitted by the operator after the prior phase's REVIEW
   approval. This is the same pattern the existing
   `workspace-attach` / `workspace-attach-ui` job split uses.
3. **Why not just bump the cap to $300 / 4h and let it run?**
   Because a single cap-bounded run that crosses 3+ REVIEW gates
   gives the operator no chance to inspect intermediate phase
   output before the runner barrels into the next. The SCOPE's
   own phasing exists for this reason — Phase 2 depends on Phase
   1's *approved* shape, not just compiled shape. REVIEW gates
   between phases are load-bearing; the runner cannot self-approve
   them.
4. **Cap stays at 30000¢ / 4h.** That cap is appropriate for
   Phase 1 alone; the iot pack is small, the contracts lift is
   mechanical, the sqlite layer is single-table. If Phase 1
   genuinely exceeds the cap, that's a signal the contracts lift
   is more invasive than the audit (Q3) predicts, not a signal to
   bump the budget.

**How to apply.** The runner reads this resolution at the top of
stage 1, prints "Phase 1 only; halt at REVIEW gate" to its own
chat, and does not start any stage past stage 2. The
template-level stages 3–7 stay in the file as documentation; the
runner treats them as `[ ]` and unreachable for this job.

### Q2 — `rule.*` node-kind registration shape

**Answer: registration uses the existing `feature` + `builtin_descriptors()` pattern in [`starter-flow-nodes`](/home/user/code/rust/starter/crates/starter-flow-nodes), exactly as the existing built-ins do. The implementations live in `starter-insights`, but the descriptors and `cfg(feature = "...")`-gated module declarations live in `starter-flow-nodes`. No circular dep, no inverted arrow.**

Ground truth from
[`crates/starter-flow-nodes/src/node_registry.rs`](/home/user/code/rust/starter/crates/starter-flow-nodes/src/node_registry.rs)
and
[`crates/starter-flow-nodes/Cargo.toml`](/home/user/code/rust/starter/crates/starter-flow-nodes/Cargo.toml):

- `starter-flow-nodes` already exposes `builtin_descriptors() -> Vec<&'static NodeDescriptor>` which conditionally pushes each kind under `#[cfg(feature = "...")]`. Adding `rule-rust` and `verdict-join` features (Phase 1) is one block of `cfg(feature)` in `lib.rs` plus one push per kind in `node_registry.rs`. Same template the existing `transform`, `branch`, `gate`, `merge`, `subflow` kinds follow.
- `starter-flow-nodes` already depends on `starter-spi` (line 29 of its `Cargo.toml`). Adding `starter-insights` as an **optional, feature-gated dependency** on `starter-flow-nodes` is the seam: `rule-rust = ["dep:starter-insights"]`, gated module declaration `#[cfg(feature = "rule-rust")] pub mod rule_rust;` whose body calls into a public function exported by `starter-insights` for the rule body.
- `starter-insights` depends on `starter-flow-spi` (for `NodeBehavior`) and `starter-spi` (for `Rule`, `Verdict`). It does **not** depend on `starter-flow-nodes` — the arrow is `flow-nodes → insights`, not the other way around. No circular dep.
- For Phase 1, the only nodes contributed are `rule.rust` and `verdict.join`. The other four rule kinds (`rule.sql`, `rule.rhai`, `rule.derive`, `rule.ai-check`, `rule.ai-debug`) and the windowing/align nodes follow the same template in later phases — same `dep:starter-insights` cargo feature gate per kind.
- **No `register_insights_nodes()` host-call function is needed.** The existing `builtin_descriptors()` slice already handles "consumer opts in via cargo feature"; insights' kinds compose into the same slice. The earlier draft proposed `register_insights_nodes()` as a fallback in case of circular dep — there is no circular dep, so the fallback is moot.

**How to apply.** In stage 1, when adding `rule.rust` and
`verdict.join`:
1. Add `rule-rust = ["dep:starter-insights"]` and
   `verdict-join = ["dep:starter-insights"]` features to
   `starter-flow-nodes`'s `Cargo.toml`. Add `starter-insights =
   { workspace = true, optional = true }` under
   `[dependencies]`.
2. Add `#[cfg(feature = "rule-rust")] pub mod rule_rust;` and
   `#[cfg(feature = "verdict-join")] pub mod verdict_join;` to
   `lib.rs`. Each module exports a `pub static DESCRIPTOR:
   NodeDescriptor` and a body that calls into
   `starter_insights::nodes::rule_rust::execute(...)` (or
   `verdict_join::execute(...)`).
3. Add the two pushes to `builtin_descriptors()` under matching
   `#[cfg(feature = "...")]` blocks, in the same place the
   existing kinds live.
4. Extend the `all-kinds` aggregate at line 96 to include the
   two new features so `cargo check --features=all-kinds` covers
   them.

No SPI change required; `NodeDescriptor` and `NodeBehavior` are
already the shared seams.

### Q3 — `starter-spi` contract churn from D1's additions

**Answer: the additions are pure additive new modules. No existing public symbol changes shape, no existing trait gains required methods, no downstream public API breaks.**

Ground truth from
[`crates/starter-spi/src/`](/home/user/code/rust/starter/crates/starter-spi/src/):
the crate is already module-organised
(`ai/`, `auth/`, `authz/`, `changelog/`, `dto/`, `error/`,
`filter/`, `i18n/`, `id/`, `paging/`, `preferences/`,
`secrets/`, `service/`, `sort/`, `tool/`, `ui/`, `units/`).
**30+ downstream crates depend on `starter-spi` today**:
`starter-agent-log`, `smoke-tests`, `starter-auth-token`,
`starter-cli`, `starter-audit`, `starter-auth-users`,
`starter-grpc`, `starter-flow-nodes`, `starter-changelog`,
`starter-auth-oauth`, `starter-changelog-postgres`,
`starter-export`, `starter-changelog-sqlite`,
`starter-service-telegram`, `starter-flow-surfaces`,
`starter-clipboard-postgres`, `starter-client-rs`,
`starter-authz`, `starter-flow-spi`, `starter-flow`,
`starter-i18n`, `starter-secrets-keyring`, `starter-prefs`,
`starter-server`, `starter-skills`, `starter-mcp`,
`starter-service-slack`, `starter-undo`, `starter-tool-slack`,
`starter-secrets-file`, and more.

That is a wide blast radius **if** the additions touch existing
shapes. They do not:

1. **New top-level modules only.** D1's symbols (`Rule`, `Verdict`,
   `Coverage`, `Dataset`, `RuleId`, `RuleSchema`, `RuleError`,
   `Severity`, `QualityFlag`, `QualityFlagId`, `Tags`, `Window`,
   `TimeZoneId`) all live in **two new modules**: `rule/` (trait,
   ids, errors, schema, severity) and `verdict/` (verdict, coverage,
   dataset, dataset-rows trait + `VecDatasetRows`, tags, window,
   timezone, quality-flag). Both are new files; nothing existing is
   touched.
2. **No existing symbol gains a new shape.** `Tool`, `AiRunner`,
   `Cancel`, `Principal`, `SecretStore`, the `dto` types, the `error`
   types — none of these change. The existing surface stays
   byte-for-byte stable.
3. **No `pub use` sweep.** D1 explicitly says "defined there, not
   re-exported". The new modules expose their types directly; nothing
   re-exports from a different crate, nothing in `starter-spi`'s
   `lib.rs` gets a new wildcard.
4. **No new trait method on an existing trait.** The new traits
   (`Rule`, `DatasetRows`) are *new* traits in *new* modules. No
   existing trait (`Tool`, `AiRunner`, …) gains a required method
   that would force every downstream impl to add a stub.

**Risk surface.** The only way the additions can break a
downstream crate is a **name collision** — a downstream that has
its own `pub struct Verdict` would shadow ours if it `use
starter_spi::*`. Mitigation:
- `grep -rln 'starter-spi'` in stage 1 to enumerate the 30+
  crates, then `rg 'struct (Rule|Verdict|Coverage|Dataset|Tags|Window|Severity|QualityFlag)\b'` in those crates to confirm zero existing collisions.
- If a collision exists (it almost certainly does not — these are
  insights-domain names), surface in chat **before** the contracts
  land; the resolution is to rename the new symbol, not to
  pre-emptively rename downstream code.

**How to apply.** Stage 1 adds two new files to `starter-spi`:
- `crates/starter-spi/src/rule/mod.rs` (sub-files: `id.rs`,
  `trait.rs`, `error.rs`, `schema.rs`, `severity.rs`).
- `crates/starter-spi/src/verdict/mod.rs` (sub-files: `verdict.rs`,
  `coverage.rs`, `dataset.rs`, `tags.rs`, `window.rs`,
  `quality_flag.rs`, `timezone.rs`).

Plus the two `pub mod rule;` and `pub mod verdict;` declarations in
`lib.rs`. No other change to `starter-spi`. Run `cargo check
--workspace` after the additions land; the build should be green
on the first try because nothing existing was touched.

## References

- Source SCOPE (authoritative):
  [/home/user/code/rust/starter/DOCS/Insights/SCOPE.md](/home/user/code/rust/starter/DOCS/Insights/SCOPE.md)
- Flow engine SCOPE: `starter/DOCS/flow/scope/SCOPE.md`
- Agent SCOPE: `starter/DOCS/agent/SCOPE.md`
- Extensions SCOPE: `starter/DOCS/extensions/scope/SCOPE.md`
- `starter-flow-nodes` registration template (Q2 ground truth):
  [/home/user/code/rust/starter/crates/starter-flow-nodes/src/node_registry.rs](/home/user/code/rust/starter/crates/starter-flow-nodes/src/node_registry.rs)
- `starter-spi` module layout (Q3 ground truth):
  [/home/user/code/rust/starter/crates/starter-spi/src/](/home/user/code/rust/starter/crates/starter-spi/src/)
- Agent rules: `starter/CLAUDE.md` (if present at root) and the
  workspace-level `codeless-workspace/CLAUDE.md` for mani / commit
  discipline.
