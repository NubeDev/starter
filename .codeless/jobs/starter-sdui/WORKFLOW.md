# Workflow — starter-sdui

How to drive this job. The shape is: "port the Rubix SDUI
substrate (`rubix-contracts/ui-ir`,
`rubix-ui-core/src/sdui/`, `extension-sdk/sdui-builder`) into
starter as five distinct artifacts (three Rust crates, one
routes crate, one React package), substitute starter-ui-kit
shadcn primitives for Rubix's UI primitives, apply two
substantive divergences (D1 form_errors→diagnostics, D2 render
target swap), and enforce three structural CI gates (dep-tree
denylist, words.txt allowlist, size budget) that didn't exist
in Rubix's pipeline."

## Sequencing

- **Stage 1 is prose-only.** Pin the five open questions
  (S-D1, S-D2, S-D3, S-D4, S-D5) in
  [`DOCS/frontend/sdui/SCOPE.md`](../../../DOCS/frontend/sdui/SCOPE.md)
  under a new `## Decisions` section. **Do not edit any other
  SCOPE in this stage** — the agent SCOPE, the ai-builder
  SCOPE, and the theme README all stay byte-for-byte.
- **Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5** form the
  core substrate; land them in order. Each phase commits +
  pushes per the closing trio. Three CI gates land with Phase
  1 so they gate every later phase from day one:
  1. Dep-tree denylist on `starter-ui-ir`.
  2. Structural allowlist (`words.txt`) on
     `crates/starter-ui-ir/src/`. Bindings + React allowlists
     land with their respective phases.
  3. The "no `starter-server` → `starter-sdui-routes` dep"
     check lands with Phase 5.
- **REVIEW after Phase 5** — the core substrate (`Component`
  IR, `EntityGraph` trait, builder DSL, `Renderer` dispatcher,
  registries, routes, capability handshake, R8 limits with
  `what:` tags) is frozen. The remaining phases (6, 7, 8)
  build on top and **must not feed back into core shape
  changes**. If a real need surfaces during Phase 6/7/8, stop
  and propose the core change explicitly — do not back-door
  it.
- **Phase 6 + Phase 7 + Phase 8** can land in any order after
  the REVIEW gate; they are largely independent. Phase 6
  (extended IR components) is additive to the registry. Phase
  7 (custom escape hatch) wires the existing registry plus
  server-side capability filter. Phase 8 (optimistic hints +
  diagnostics + full DoS enforcement + falsification suite)
  closes the safety surface.
- **Stage 12 (smoke tests + structural enforcement gates)** is
  the merge gate. No phase ships individually without its own
  subset of the smokes passing in CI; the full sweep gates the
  final merge.

## Per-stage discipline

- **Before any code change in a phase:**
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in
    [`DOCS/frontend/sdui/SCOPE.md`](../../../DOCS/frontend/sdui/SCOPE.md)
    that the stage touches. R1 through R9 are the load-bearing
    ones; if a change makes any of them harder to enforce,
    **stop and write up the conflict** in the handover before
    continuing.
  - Re-read the **Rubix source file(s)** the phase ports from.
    The Rubix references in
    [`DOCS/frontend/sdui/SCOPE.md`](../../../DOCS/frontend/sdui/SCOPE.md)
    "Pointers" section name them per phase. The ports are
    largely copy-paste with renames; reading the source first
    catches divergences the SCOPE didn't anticipate.
  - For Phase 1, re-read SCOPE R1 (zero I/O) and R2 (versioned
    IR + dual-field tolerance). D1 (form_errors → diagnostics)
    lands here; the action response type signatures change.
  - For Phase 2, re-read SCOPE R4 (server-side bindings) and
    the "Data bindings" worked example. The `EntityGraph`
    trait shape is the S-D1 decision from stage 1; honour it.
    The optional `entity_id_regex` method on the trait is
    needed by ai-builder R7 — include it from day one.
  - For Phase 3, re-read the M4 fix in SCOPE "Surface — Rust
    (builder DSL)" — `starter-ui-builder` depends on
    `starter-ui-ir` only, NOT on `starter-ui-bindings`.
    Document the compile-time-vs-resolve-time contract
    verbatim in the crate-level docs.
  - For Phase 4, re-read SCOPE "Surface — React" and the size
    budgets (`Renderer.tsx` ≤ 800 lines TSX single file,
    components ≤ 4000 lines red line total). The size-budget
    CI gate lands here.
  - For Phase 5, re-read SCOPE "Surface — Rust (HTTP routes,
    opt-in)" and the M6 fix. **`starter-server` does NOT
    depend on `starter-sdui-routes`.** The structural CI gate
    that enforces this lands here.
  - For Phase 6, re-read SCOPE "Streaming content" — the
    stream sentinel naming (S-D5) is inherited from Rubix
    verbatim. The chart `sources: Vec<ChartSource>` dual-field
    tolerance (V5 migration note) is load-bearing.
  - For Phase 7, re-read SCOPE R7 (capability handshake threat
    model). `renderer_id` is **public**; `custom.props`
    authorisation runs at the handler / resolve boundary; the
    capability filter is vocabulary, not auth. Document this
    in the crate-level docs.
  - For Phase 8, re-read SCOPE R8 evidence column (per the M3
    fix). Each limit row says `Inherited / unmeasured`,
    `Inherited`, or `Reused`. Do not silently widen any limit;
    if a fixture requires a wider value, that's the trigger to
    measure and update the row, not to bump the constant.
- **Touch only what the stage names.** No drive-by refactors.
  If the renderer needs a helper that does not yet exist in
  `@nube/starter-ui-kit`, **stop and write up the gap** in the
  handover rather than reaching across packages.
- **Verify before commit:**
  - **Rust**: `cargo check --workspace --all-features
    --all-targets`, then `cargo test -p <touched-crate>`, then
    `cargo clippy --workspace --all-targets -- -D warnings`.
  - **TS**: `pnpm -r build` from workspace root, then `pnpm -r
    test`, then `pnpm -r lint`.
  - **Dep-tree gate** (every stage that touches
    `starter-ui-ir`): `cargo tree -p starter-ui-ir --edges
    normal` must not contain `axum`, `axum-core`, `reqwest`,
    `hyper`, `tokio`, `tokio-util`, `tower`, `tower-http`,
    `h2`, `http-body`. If any appears, an I/O dep has leaked —
    stop and debug, do not commit.
  - **Structural allowlist gate** (every stage that touches
    `crates/starter-ui-ir/src/`,
    `crates/starter-ui-bindings/src/`, or
    `packages/starter-sdui-react/src/`): the words.txt scanner
    must report zero unlisted tokens. If a new word legitimately
    appears (e.g. a new IR variant name), add it to the
    relevant `words.txt` in the same commit with a one-sentence
    justification in the PR description.
  - **No-`starter-server`-dep gate** (every stage in Phase 5+):
    `cargo tree -p starter-server --edges normal | grep
    starter-sdui-routes` must return empty. If the consumer-
    opt-in claim from M6 breaks, the M6 fix is unwound —
    structural, not advisory.
  - **Size-budget gate** (every stage that touches
    `packages/starter-sdui-react/src/`): `Renderer.tsx` line
    count ≤ 800; total component file lines ≤ 4000. Library
    code (tiptap, monaco-diff) is excluded — only the
    IR-adapter wrappers count.
  - **Smoke harness** (every stage that lands a smoke): run
    the new smoke locally before committing. A stage is not
    done until its smoke is green.
- **Commit only if green.** One logical batch per commit;
  commit message stage-tagged: `stage N: <one-line title from
  template.yaml>`.

## REVIEW gates

Two:

- **After Stage 1** — decisions sign-off before any code
  lands. The five open questions (S-D1, S-D2, S-D3, S-D4,
  S-D5) must be recorded under
  [`DOCS/frontend/sdui/SCOPE.md`](../../../DOCS/frontend/sdui/SCOPE.md)
  `## Decisions` with revisit triggers. The bias notes in the
  per-job SCOPE.md §"Open questions" are the starting point —
  override only if the runner finds a concrete reason.
- **After Phase 5** — the core substrate is frozen.
  `Component` IR shape, `EntityGraph` trait, builder DSL
  surface, `Renderer` dispatcher, `HandlerRegistry` /
  `QueryEngine` traits, and the three `/ui/*` routes must not
  change shape past this point. Phases 6, 7, 8 build on top.

Write a one-line summary into the handover at each gate. Do
not proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | `DOCS/frontend/sdui/SCOPE.md` has a `## Decisions` section with S-D1 (`EntityGraph` in `starter-ui-bindings`), S-D2 (trait + in-memory ref impl), S-D3 (defer visual editor), S-D4 (`oneOf` helper on renderer side), S-D5 (inherit Rubix sentinel verbatim). No code changed. |
| 3 | `starter-ui-ir` member compiles in the workspace; JSON Schema artifact emits deterministically; `diagnostics` variant present, `form_errors` rejected; dep-tree CI gate green; D1 + D3 entries in `DIVERGENCE.md`. |
| 4 | `starter-ui-bindings` compiles; "one page, N targets" smoke green against fixture entity graph; `EntityGraph` trait has the optional `entity_id_regex` method; D5 entry in `DIVERGENCE.md`. |
| 5 | `starter-ui-builder` compiles; `RowsSource` → `line_chart` is a build error; worked example page from `main.rs` renders end-to-end against fixture data; "Builder DSL produces valid IR" smoke green; compile-time-vs-resolve-time contract documented in crate-level docs. |
| 6 | `@nube/starter-sdui-react` builds; 16 components dispatch through `<Renderer>` and render against `starter-ui-kit` primitives; `Renderer.tsx` ≤ 800 lines; components total under 3000-line target; capability handshake refuses V+1 tree; D2 entry in `DIVERGENCE.md`. |
| 7 | `starter-sdui-routes` crate compiles standalone; `sdui_router(state)` mounts the three routes; `HandlerRegistry` + `QueryEngine` traits compile; R8 limits return 413 with stable `what:` tags; no-`starter-server`-dep gate green; D4 entry in `DIVERGENCE.md`. |
| 9 | Extended IR components (chart, sparkline, tree, timeline, markdown, wizard, drawer, rich_text, diff, ref_picker, date_range) compile + render; streaming subscriptions work on text/markdown/code/timeline with the `stream_end` sentinel; `$page.chart_range` round-trip triggers re-resolve; chart `source`/`sources` dual-field tolerance preserved. |
| 10 | `registerCustomRenderer` registry exported from `@nube/starter-sdui-react`; server-side capability filter rewrites unfamiliar `renderer_id` to `dangling`; "Custom renderer falls back cleanly" smoke green; capability-handshake threat model documented. |
| 11 | `Action.optimistic` round-trip + rollback works; `diagnostics` response shape end-to-end; all seven R8 limits enforced with stable `what:` tags; falsification suite (CRUD + diff + state-board) green with zero domain strings in renderer crates. |
| 12 | All ten SCOPE smoke tests green in CI: domain-leak structural check, one-page-N-targets, capability mismatch, action 404 structured, table pagination round-trip, custom fallback, R8 `what:` tags (each), builder DSL valid IR, falsification suite, no-`starter-server`-dep. Plus dep-tree gate, structural allowlist gate, size-budget gate all green. |

## Anti-patterns

- **Inventing a parallel UI wire format.** SCOPE — SDUI is the
  one structured UI format in starter. ai-builder targets this
  IR. If a future feature feels like it wants a sibling format,
  stop and write up why the existing IR doesn't cover it; the
  answer is almost certainly a new IR variant or a `custom`
  renderer-id, not a parallel format.
- **A `Cargo.toml`-regex grep instead of `cargo tree`.** R1 —
  the grep misses the common failure mode (a serialisation
  crate pulls an I/O dep through a feature flag). The dep-tree
  denylist is the contract.
- **A keyword denylist instead of the words.txt allowlist.**
  R3 — a denylist passes silently for any consumer whose
  domain isn't on it. The allowlist is the contract; the
  denylist is defence-in-depth.
- **A `feature = "sdui"` flag on `starter-server`.** M6 —
  Cargo features cannot prevent workspace crates from being
  built if anything depends on them. The only honest opt-out
  is a separate crate the consumer adds to their own
  Cargo.toml.
- **`starter-ui-builder` depending on `starter-ui-bindings`.**
  M4 — binding strings are resolve-time errors, not
  compile-time. Adding the dep would couple two crates the
  split exists to keep separate; the "tool-only consumer
  pulls just ir + builder" claim breaks.
- **`form_errors` back-compat shim.** D1 — starter has not
  shipped; we drop it at the wire. A handler emitting
  `form_errors` is a parse error. Adding a deserialiser
  fallback re-introduces the maintenance burden Rubix carried
  for one release.
- **Rendering against Rubix's primitives.** D2 — render
  against `@nube/starter-ui-kit` shadcn primitives. Importing
  a parallel UI library defeats the theme editor.
- **A `custom.props` payload that varies by principal at the
  capability filter.** R7 threat model — capability filter is
  vocabulary, not auth. Authorisation runs at the handler /
  resolve boundary. A renderer whose props contain secrets
  when rendered for any plausible principal is misconfigured
  at the source.
- **Widening an R8 limit silently.** R8 — the evidence column
  in the SCOPE table classifies each limit; widening requires
  measurement and an evidence column update in the same PR.
  Bumping a constant because a fixture hit it is the wrong
  reflex.
- **Adding a `renderer_id` lookup that touches storage on
  every resolve.** R8 implicit — the capability filter and
  registry are in-memory. Storage on the hot path multiplies
  resolve tail latency.
- **A drag-drop visual editor inside `@nube/starter-sdui-react`.**
  S-D3 — defer. Lands as a starter-extension when a consumer
  demands. Building it in here grows the renderer past its
  budget and couples the dumb projector to authoring.
- **Editing files in `/home/user/code/rubix-workspace/`.**
  Rubix is the origin, not a maintained fork. Ports go
  one-way; if Rubix evolves, we cherry-pick and add a
  DIVERGENCE row.
- **Skipping `DIVERGENCE.md` updates.** Every divergence lands
  in the same PR as the code change. Drift between
  `DIVERGENCE.md` and reality re-introduces the "Rubix doc is
  authoritative" confusion that motivated DIVERGENCE.md
  existing.
- **Adding domain-specific strings to the words.txt allowlist
  without justification.** R3 enforcement — the allowlist is
  the contract. Adding a word requires one sentence in the PR
  description naming the framework concept. "Convenience" is
  not a framework concept.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items,
in order. The user watches these tick over in the `Stages`
overview; they are how the user confirms a long-running stage
actually landed instead of just looking like it did. Do
**not** rename or reorder them.

1. `checks` — run the stage's verify list. Every step must
   pass. On failure: stop, fix, re-run; do not advance to
   `docs`.
2. `docs` — update `handover.md` for the next stage and the
   active session doc, in the same worktree, so the fresh
   agent that opens the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push
   to the job's branch (`codeless/starter-sdui`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook
fails, fix the cause.
