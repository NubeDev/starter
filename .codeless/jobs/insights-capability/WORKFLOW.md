# Workflow — insights-capability

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md` and the authoritative source SCOPE at
[/home/user/code/rust/starter/DOCS/Insights/SCOPE.md](/home/user/code/rust/starter/DOCS/Insights/SCOPE.md).

## Sequencing

This is one big job mirroring the source SCOPE's four phases. The
stages are **strictly linear**: Phase 2 depends on Phase 1's
`Rule` trait + registry + persistence; Phase 3 depends on Phase 2's
windowing + rollups; Phase 4 depends on Phase 3's `align` + cache.
Do not start a later stage until the prior REVIEW gate clears.

The three REVIEW gates exist because each phase changes the public
surface of `starter-spi` and `starter-flow-nodes` enough that a
silent merge of the wrong shape costs a phase to undo. At each gate:

- Phase 1 (after stage 1): confirm dep arrows, R-ins-9 invariant
  (no parallel orchestrator), `contributes.tools` registration shape.
- Phase 2 (after stage 3): confirm D5 retroactive flag emission,
  per-window rollup watermark, Rhai sandbox rejection of every item
  R-ins-4 lists.
- Phase 3 (after stage 5): confirm every LLM call routes through
  `AiRunner` (synthesise a violator and prove the dep-tree gate
  fails), confirm the bills-reconciliation row passes including
  retroactive flagging on a tariff fixup.

## Per-stage discipline

Before writing any code in a stage:

1. Re-read the corresponding **Phase** in the source SCOPE
   §"Phasing". The phase text is the contract; this WORKFLOW is the
   process.
2. Re-read `SCOPE.md` §"In scope" and §"Out of scope". The biggest
   risk on this job is silent scope creep — the source SCOPE is
   broad and tempting to expand into. Do not.
3. For stages that touch `starter-spi`: confirm no existing
   downstream crate's public API breaks. New symbols are fine; a
   rename or re-shape is a stop-and-surface event.
4. `cargo check --workspace` from the `starter` repo root before any
   edit so the baseline is known-clean.

Before committing a stage:

1. `cargo fmt --all` clean.
2. `cargo clippy --workspace --all-targets -- -D warnings` green.
3. `cargo test --workspace` green.
4. The phase's reference smoke (IoT / Energy / HVAC+bills / Finance)
   passes locally. If the smoke harness does not exist yet (Phase 1
   creates it), the stage that introduces a smoke also runs it.
5. Update `SCOPE.md` §"Deliverables" with `[x]` against anything
   completed in the stage.
6. Update `starter/DOCS/Insights/SCOPE.md` §"Phasing" with the
   matching `[x]` for the phase landed.

Commit + push via **mani** from the codeless-workspace root:

```
./bin/mani --config mani.yaml run commit --projects starter \
  MSG='stage N: <one-line title from template.yaml>'
./bin/mani --config mani.yaml run push --projects starter
```

No `--force`, no `--no-verify`. If a hook fails, fix the cause.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. Do not rename or reorder.

1. `checks` — `cargo fmt --check` + `cargo clippy --workspace
   --all-targets -- -D warnings` + `cargo test --workspace` + the
   phase smoke. Every step must pass. On failure: stop, fix, re-run.
2. `docs` — update `handover.md` for the next stage and the active
   session doc, and tick the relevant `[x]` in
   `starter/DOCS/Insights/SCOPE.md` §"Phasing".
3. `git` — stage the changes, commit with `stage N: <title>`, push
   to `codeless/insights-capability`. Stage and commit one phase at
   a time; do not bundle two phases.

A stage is not "done" until all three are green and the push
succeeds.

## REVIEW gates

Three gates: stage 2 (Phase 1 complete), stage 4 (Phase 2
complete), stage 6 (Phase 3 complete). At each gate, write a
handover comment in the job chat containing:

- One bullet per item the gate is checking (taken from §"REVIEW"
  in the stage description).
- For Phase 1: the dep-arrow audit summary (which crates gained
  symbols, which crates' public APIs were *not* broken, the
  `register_insights_nodes()` shape).
- For Phase 2: the D5 retroactive flag test transcript and the
  R-ins-4 sandbox-rejection list (every item explicitly tested).
- For Phase 3: the dep-tree-gate violator test transcript and the
  bills-reconciliation row end-to-end smoke output.

Do not proceed past a REVIEW gate without explicit approval in
chat.

## Anti-patterns specific to this job

- **Do not** create `starter-insights-spi`, `starter-insights-rhai`,
  `starter-insights-sql`, or `starter-insights-store`. R-ins-1 is
  non-negotiable; the seams that would justify a split are already
  owned by `starter-spi` (contracts) and `starter-store-*`
  (persistence).
- **Do not** build a second orchestrator inside `starter-insights`.
  R-ins-9 binds. If a stage needs branching / retry / fan-in /
  error routing, **use the existing flow engine nodes** (`branch`,
  `gate`, `retry`, `verdict.join`). If those nodes are missing
  features the SCOPE demands, that's a flow-engine job, not this
  one — surface in chat and stop.
- **Do not** introduce a second LLM seam. Every model call routes
  through `AiRunner`. The dep-tree gate added in stage 3 is the
  enforcement; the discipline is older. Importing an SDK directly
  inside a rule pack is a halt-and-rework event.
- **Do not** make `Rule`, `Verdict`, or `Dataset` heavier in
  `starter-spi` than D1 allows. `Dataset::rows` is
  `Arc<dyn DatasetRows>`; `VecDatasetRows` is the only impl in
  `starter-spi`; `StreamingDatasetRows` lives in
  `starter-insights`. A pack that needs more depends on
  `starter-insights`, not on a widened `starter-spi`.
- **Do not** widen `RuleOutput` to a third variant. The two-shape
  rule (`Verdict` vs `Dataset`) is load-bearing for R-ins-7.
  Things that don't fit (e.g. `align`'s `Frame` output, D8) stay as
  **flow-nodes**, not as rules.
- **Do not** ship operator UI. Even a "tiny one" leaks into the
  SCOPE §"Non-goals" — `starter-ui-*` is consumer-owned and ships
  separately.
- **Do not** silently land a partial phase. If a stage cannot
  complete inside the runner's cost or wall-clock cap, mark the
  stage `[!]` in `SCOPE.md` and stop (per the inner-repo CLAUDE.md
  rule against half-finished implementations). Do not commit a
  TODO-laced partial.
- **Do not** drift the SLO targets in stage 4. D9 numbers are
  budgets, not promises, but the budget is fixed for this job. If
  the SLO smoke fails, the fix is to tighten the query, the index,
  or the rollup schema — never to relax the budget in the same PR
  that exposes the regression.
- **Do not** introduce a backfill bigger than the D3 100k cap.
  `BackfillTruncated` is the contract; the tuner agent reads it.
  Removing the cap is a separate scope-doc decision, not a stage-2
  patch.
- **Do not** expand rule-pack contributions to non-rule kinds in
  Phase 1–3. D7 allows custom node kinds via packs, but none of the
  four reference packs in this job need them.

## When to halt

- The "scope realism" check at the top of stage 1 finds the cost
  cap is materially lower than the work required for even Phase 1.
  Surface in chat, propose splitting the job into per-phase
  sub-jobs, do not start coding.
- The `starter-spi` contract audit (Q3 in `SCOPE.md` §"Open
  questions") finds a downstream crate whose public API breaks
  under D1's additions. Halt and surface; the resolution may need
  a `starter-spi` major-bump or a sister-crate facade, neither of
  which is in scope without explicit approval.
- A phase smoke (IoT / Energy / HVAC+bills / Finance) cannot pass
  without a flow-engine change. The flow engine is not this job's
  scope; surface in chat and either get the flow-engine change
  done as a precursor or pause this job until it lands.
- The CI dep-tree gate in stage 3 cannot be made to fail on the
  synthesised violator. That means the gate is not actually
  enforcing R-ins-5 — halt and rework before pretending coverage
  exists.
- Any R-ins-1 / R-ins-7 / R-ins-9 / R-ins-10 violation in code
  review. These rules are written into the SCOPE precisely so a
  later reader doesn't have to argue around them; respect them in
  the original direction.
