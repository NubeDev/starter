# Workflow — starter-skills

How to drive this job. The shape is: "the `SkillSelector` seam is
already wired and tested; this job ships the missing
implementation behind it — a parser, an agent-R4 content-hash
algorithm, a quarantine state machine, an `ApprovalStore`, three
selectors, an on-mount hash check in the `ai-agent` node, the
`starter-ext-flow` wiring, and two reference bundles."

## Sequencing

- **Stage 1 is prose-only.** Pin the four open questions (S-D1,
  S-D2, S-D3, S-D5) in [`DOCS/agent/SKILLS.md`](../../../DOCS/agent/SKILLS.md)
  under a new `## Decisions` section, then commit. **Do not edit
  `DOCS/agent/SCOPE.md`** — R4 stays byte-for-byte as specified.
- **Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 4b** form the
  core crate; land them in order. Each phase commits + pushes per
  the closing trio. The dep-tree CI gate (no provider SDK in
  `starter-skills`) lands with Phase 1 so it gates every later
  phase from day one.
- **REVIEW after Phase 4b** — the core surface
  (`SkillRegistry`, `ApprovalStore`, three selectors, on-mount
  hash check, six core smokes) is frozen. The remaining phases
  (5, 6, 7) build on top and **must not feed back into core
  shape changes**. If a real need surfaces during Phase 5/6/7,
  stop and propose the core change explicitly — do not
  back-door it.
- **Phase 5 + Phase 6 + Phase 7** can land in any order after the
  REVIEW gate; they are independent. Phase 5 (store impls) is
  the only one that touches another crate's migration story;
  Phase 6 (ext-flow wiring) is a single-file branch in the
  existing adapter; Phase 7 (reference bundles) is two
  `SKILL.md` files lifted verbatim from ai-builder SCOPE.
- **Stage 12 (smoke tests)** is the merge gate. No phase ships
  individually without its own subset of the nine smokes passing
  in CI; the full sweep gates the final merge.

## Per-stage discipline

- **Before any code change in a phase:**
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in
    [`DOCS/agent/SKILLS.md`](../../../DOCS/agent/SKILLS.md) that
    the stage touches. R-skills-1 through R-skills-8 are the
    load-bearing ones; if a change makes any of them harder to
    enforce, **stop and write up the conflict** in the handover
    before continuing.
  - For Phase 1, re-read the source SKILLS.md §"Public API surface
    (v1)" — the snippet there is the spec, byte-for-byte.
  - For Phase 2, re-read SKILLS.md R-skills-2 — the six-step
    algorithm is normative. The collision-prevention framing
    (length-prefixed, not `\0`-separated) and the exact
    line-ending byte transforms (CRLF → LF then lone CR → LF,
    no BOM stripping) are load-bearing.
  - For Phase 3, re-read SKILLS.md R-skills-3 (trust matrix) and
    R-skills-7 (`ApprovalStore` is append-mostly). Drift on
    reload **never** mutates the store.
  - For Phase 4, re-read SKILLS.md R-skills-5 (failure
    semantics: 2 s timeout, no retries, fail-to-None, metric +
    WARN log + tracing span on every outcome). The graceful
    degradation rule is non-negotiable — a transient provider
    hiccup must never block a flow run.
  - For Phase 4b, re-read SKILLS.md §"How it plugs into the
    engine" — the four-step quarantine-under-reload invariant
    depends on the on-mount hash check. The smoke
    ("Resource hash mismatch aborts the run") proves it; the
    `ai-agent` node modification is the only place this check
    can live.
  - For Phase 5, re-read SKILLS.md R-skills-7 (`ApprovalStore`
    trait shape). One new table per store crate; existing
    migration conventions apply.
  - For Phase 6, re-read agent SCOPE R-agent-4 (extensions
    contribute via `starter-ext-flow`) and SKILLS.md
    §"Relationship to existing crates" (no `starter-ext-skills`
    crate — one new branch in the existing adapter).
  - For Phase 7, the two reference bundles' content is lifted
    **verbatim** from
    [`DOCS/frontend/ai-builder/SCOPE.md`](../../../DOCS/frontend/ai-builder/SCOPE.md)
    §"Skills for ai-builder". Do not paraphrase — copy.
- **Touch only what the stage names.** No drive-by refactors. If
  the parser needs a helper that does not yet exist in
  `starter-flow-spi`, **stop and write up the gap** in the
  handover rather than reaching across crates.
- **Verify before commit:**
  - **Rust**: `cargo check --workspace --all-features
    --all-targets`, then `cargo test -p starter-skills` (or the
    touched crate), then `cargo clippy --workspace
    --all-targets -- -D warnings`.
  - **Dep-tree gate** (every stage that touches
    `starter-skills`): `cargo tree -p starter-skills --edges
    normal` must not contain `async-openai`, `anthropic-ai-sdk`,
    `anthropic-sdk`, `google-genai`, `aws-sdk-bedrockruntime`,
    `mistralai`, `ollama-rs`. If any appears, the
    `LlmSkillSelector` has leaked a provider dep — stop and
    debug, do not commit.
  - **Smoke harness** (every stage that lands a smoke): run the
    new smoke locally before committing. A stage is not done
    until its smoke is green.
- **Commit only if green.** One logical batch per commit;
  commit message stage-tagged: `stage N: <one-line title from
  template.yaml>`.

## REVIEW gates

Two:

- **After Stage 1** — decisions sign-off before any code lands.
  The four open questions (S-D1, S-D2, S-D3, S-D5) must be
  recorded under
  [`DOCS/agent/SKILLS.md`](../../../DOCS/agent/SKILLS.md)
  `## Decisions` with revisit triggers. The bias notes in the
  per-job SCOPE.md §"Open questions" are the starting point —
  override only if the runner finds a concrete reason.
- **After Phase 4b** — the core crate surface is frozen.
  `SkillRegistry`, `ApprovalStore`, the three selectors, and the
  `ai-agent` on-mount check must not change shape past this
  point. Phases 5, 6, 7 build on top.

Write a one-line summary into the handover at each gate. Do not
proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | `DOCS/agent/SKILLS.md` has a `## Decisions` section with S-D1 (out of scope; flag for starter-cli), S-D2 (file:// only in v1), S-D3 (best-effort pass-through with WARN log), S-D5 (keep R4 byte-for-byte). No code changed. |
| 3 | `starter-skills` member compiles in the workspace; `SKILL.md` parser passes unit tests for happy path, `deny_unknown_fields` rejection, missing-id rejection. Bundle struct holds frontmatter + body (Arc<str>) + raw resources. |
| 4 | `hash_bundle(path)` deterministic across line endings (LF / CRLF / CR-only all produce the same hash for the same logical content); path-framing collision smoke green; pinned-digest unit test green. |
| 5 | `SkillRegistry::builder()` builds; `load_dir`, `load_dir_quarantined`, `extend` all classify trust per the matrix in R-skills-3; `ApprovalStore` trait + `InMemoryApprovalStore` work; "Extension is quarantined regardless of frontmatter" and "Hash mismatch re-quarantines" smokes green. |
| 6 | Three selectors compile + unit tests pass; `SkillRegistry` implements `SkillSelector` and filters quarantined bundles before delegating; engine integration works (`Engine::with_skill_selector(Arc::new(registry))`); "Selection is frozen per run" and "Quarantined never reaches strategy" smokes green. |
| 7 | `ai-agent` node mounts resources with on-mount `blake3` verification; "Resource hash mismatch aborts the run" smoke green; "No I/O on select" smoke green (panicking `ApprovalStore` does not panic `select`). |
| 9 | SQLite + Postgres `SkillApprovalStore` impls compile behind a default-on `skill-approvals` feature; one round-trip test per backend (record / lookup / list / revoke / lookup-after-revoke); migrations land idempotent. |
| 10 | `starter-ext-flow` adapter handles `contributes.skills` with `dir:`; an integration test stubs an extension manifest, runs the adapter against a fixture extension dir, asserts the loaded skills appear in `registry.list_quarantined()` and not in `registry.list()`. |
| 11 | `skills/starter.ai-builder.dashboards/SKILL.md` and `skills/starter.ai-builder.themes/SKILL.md` land at the workspace root with content lifted verbatim from ai-builder SCOPE §"Skills for ai-builder"; an integration test loads the dir and round-trips both through `select()` with a matching query. |
| 12 | All nine smoke tests pass in CI for the new crate (line-endings, extension quarantine, hash mismatch, selection frozen, quarantined-not-in-strategy, no-I/O-on-select, resource hash mismatch, path framing collision, pinned digest). Dep-tree CI gate on `starter-skills` is green and fails the build on any provider SDK appearance. |

## Anti-patterns

- **Templating in skill bodies.** R-skills-1 — bodies and
  resources are read once and held by `Arc`. A `{{x}}` is
  literal text the model sees. Any code path that *expands*
  a placeholder in a `SKILL.md` body or resource is a CVE-class
  bug. The "no templating" smoke proves this.
- **Mutating the `ApprovalStore` from a read path.** R-skills-7
  — drift on `reload()` does not mutate the store. The prior
  row stays inert. Mutating from `select()` or `reload()`
  reintroduces I/O on the hot path (violates R-skills-8) and
  loses the audit trail of which `(skill_id, hash)` was ever
  approved.
- **Frontmatter `trust: approved` raising trust on
  `extend(...)`.** R-skills-3 row 3 — extensions are always
  quarantined regardless of frontmatter. An extension cannot
  self-approve. The "Extension is quarantined regardless of
  frontmatter" smoke proves this; a code path that honours
  frontmatter on `extend(...)` is a security regression.
- **Retries inside `LlmSkillSelector`.** R-skills-5 — no
  retries. Selection is on the hot path; retries multiply tail
  latency for a feature that gracefully degrades. A 2 s timeout
  is the hard ceiling.
- **Reading the resource file in `SkillRegistry::load_dir` and
  caching the bytes.** R-skills-2 keeps resource contents off
  the registry — only `ResourceRef { uri, content_hash }`
  lives in memory. The `ai-agent` node resolves URIs to bytes
  at mount time and verifies the hash. Eager-loading resources
  inflates the registry for the 99 % of runs that never use a
  given skill.
- **Skipping the on-mount hash check in the `ai-agent` node.**
  Phase 4b — without it, a `reload()` racing with an in-flight
  run silently mounts new bytes. The quarantine guarantee
  leaks. The "Resource hash mismatch aborts the run" smoke is
  the only thing standing between this crate's guarantee and a
  silent drift bug.
- **Adding a vendor SDK or `tokio` runtime dep to
  `starter-skills`.** The dep-tree CI gate enforces. A diff
  means either the baseline updates (a separate, reviewed
  commit) or the change is rolled back.
- **Adding a `--allow-quarantined` flag.** R-skills-4 — the
  quarantine guarantee is the reason extensions can contribute
  skills at all. There is no bypass.
- **Inventing a new SPI seam.** SKILLS.md §"Relationship to
  existing crates" — `SkillSelector` already lives in
  `starter-flow-spi`. `starter-skills` is the implementation,
  not a new seam. Any new trait in `starter-flow-spi` for this
  job is a smell; stop and write it up.
- **A `starter-ext-skills` crate.** R-agent-4 — extensions
  contribute through `starter-ext-flow`'s existing adapter.
  One new branch in the existing handler. Minting a new
  adapter is the anti-pattern.
- **Editing `DOCS/agent/SCOPE.md` R4.** S-D5 flags the possible
  "drop normalisation entirely" simplification; this job
  implements R4 as specified and flags the trade-off in
  SKILLS.md Decisions. Editing R4 in this job changes scope.
- **Lifting upstream skill-runner architecture from the prior
  `ai-ui` prototype.** The seam already exists; the
  implementation here is starter-native. The `ai-ui`
  prototype's bespoke provider trait, broadcast push channel,
  and skill format are explicitly not the model.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
   On failure: stop, fix, re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the active
   session doc, in the same worktree, so the fresh agent that opens
   the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push to the
   job's branch (`codeless/starter-skills`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
