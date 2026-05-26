# Workflow — puck-builder-finish

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md` and the two scope docs at
[/home/user/code/rust/starter/rubix/docs/scope/dashboards/10-puck-builder.md](/home/user/code/rust/starter/rubix/docs/scope/dashboards/10-puck-builder.md)
and
[/home/user/code/rust/starter/rubix/docs/scope/dashboards/11-live-canvas-sse.md](/home/user/code/rust/starter/rubix/docs/scope/dashboards/11-live-canvas-sse.md).

## Sequencing

Five stages, no REVIEW gates. Order matters:

- Stage 1 (§B3 selectors) ships the Catalogue seam pattern that
  stage 4 (runtime hash banner) and stage 5 (live-canvas SSE)
  may piggyback on for their own consumer-side fills.
- Stage 2 (tenant + discard + test cleanup) is independent and
  could in principle run first, but staying after stage 1 keeps
  the §B3 work in a clean tree without the discard-bridge
  refactor mixed in.
- Stage 3 (placeholder coverage) is independent. Run after
  stages 1–2 so the placeholder gap is computed against the IR
  schema as it lands in this branch.
- Stage 4 (runtime hash banner) consumes the build-time emit
  added in stage 4 itself; no upstream stage dependency.
- Stage 5 (live-canvas SSE) is the largest and last. It depends
  on the existing 409 modal mechanics (already landed) and
  reuses the SSE plumbing in `sdui-page.tsx`.

The user explicitly asked for an uninterrupted run. No REVIEW
gates. If something genuinely surprising surfaces (see "When to
halt" below), halt and surface — do not power through.

## Per-stage discipline

Before writing any code or docs in a stage:

1. Re-read the corresponding section of the scope doc. The scope
   text is the contract; this WORKFLOW is the process.
2. Re-read `SCOPE.md` §"In scope" and §"Out of scope". Biggest
   risk is creeping into deferred work (MCP refactor, parallel
   SSE client, hypothetical second catalogue source).
3. **Do not touch `crates/` or `rubix/crates/`.** If a Rust change
   looks required, halt at the "When to halt" list below.
4. For stage 1: warm up by listing the live agent's
   `/api/v1/tools` to discover the analytics-template / tool /
   tenant catalogue verbs. Record the verb names in the handover.
   If a required verb is missing, halt.
5. For stage 2: warm up by reading
   `packages/starter-client-react/src/` exports to find the
   session hook + its shape. If the tenant field is absent,
   halt.
6. For stage 3: warm up by walking
   `crates/starter-ui-ir/schema/starter-ui-ir.schema.json`
   (read-only — the JSON file is fair game even though `crates/`
   is frozen for edits) `definitions.Component.oneOf` against
   the existing switch in
   `packages/starter-ui-sdui-react/src/headless/placeholder-render.tsx`
   to enumerate the gap. Record the list in the handover.
7. For stage 4: warm up by hitting `/api/v1/tools` again to
   discover the schema-hash verb. If absent, halt.
8. For stage 5: re-read scope 11 in full. It is the largest stage
   and the contract is dense.

Before committing a stage:

1. `pnpm --filter @nube/starter-ui-sdui-puck typecheck` green.
2. `pnpm --filter @nube/starter-ui-sdui-puck test` green
   (includes the schema-drift CI check).
3. `pnpm --filter @nube/starter-ui-sdui-react test` green.
4. `pnpm --filter @nube/rubix-frontend typecheck` green.
5. For stages that touch the browser path (1, 2, 3, 4, 5 —
   all of them): manual verify against
   `http://localhost:5173/dashboards/data-flow-site-a/edit`.
   The expected outcome per stage is in SCOPE.md §"Deliverables"
   item 6.

Commit + push from the starter repo root:

```sh
git add -A
git commit -m "stage N: <one-line title from template.yaml>"
git push origin codeless/puck-builder-finish
```

No `--force`, no `--no-verify`. If a pre-commit hook fails, fix
the cause and re-commit as a new commit (never `--amend` here —
see the handover guidance in CLAUDE.md).

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's `verify:` list (typecheck + test +
   manual browser verify). Every step must pass. On failure:
   stop, fix, re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage, update the
   `packages/starter-ui-sdui-puck/README.md` status table to
   flip the relevant ⏳ row to ✅, and (if relevant) update the
   `packages/starter-ui-sdui-react/README.md`.
3. `git` — stage the changes (`git add -A` from the worktree
   root), commit with `stage N: <one-line title from
   template.yaml>` so the history mirrors the template stages
   one-for-one, and push to `codeless/puck-builder-finish`.

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`.

## Anti-patterns specific to this job

- **Do not** edit the scope docs (`10-puck-builder.md`,
  `11-live-canvas-sse.md`). They are locked. Disagreement goes in
  the handover.
- **Do not** edit `crates/` or `rubix/crates/`. Frozen. If a Rust
  change is required, halt — that is a Rust-side job.
- **Do not** edit `pnpm-workspace.yaml`.
- **Do not** add a new top-level npm dependency without halting
  to surface it first. Minor version bumps to existing deps are
  fine; brand-new packages are not.
- **Do not** make `@nube/starter-ui-sdui-puck` depend on any
  `rubix-*` package. The package boundary is preserved through
  seams (the §B4 save seam, the §B3 catalogue seam landing in
  stage 1). The rubix-frontend route fills the seam; the
  package consumes it.
- **Do not** invent verb names. Discover them via
  `/api/v1/tools` at stage warmup. If a required verb is
  missing, halt.
- **Do not** hardcode tenants. Stage 2's whole point is dropping
  the `"system"` literal; do not reintroduce it elsewhere.
- **Do not** stand up a parallel SSE client for stage 5. Reuse
  the plumbing in `packages/starter-ui-sdui-react/src/headless/sdui-page.tsx`.
- **Do not** leave graveyard comments ("renamed from X",
  "removed in stage Y"). The commit message carries that.
- **Do not** add error handling or fallbacks for scenarios that
  can't happen. Trust the framework guarantees. Validate only at
  system boundaries (transport, user input).
- **Do not** treat the runtime hash banner (stage 4) as gating —
  it is a non-blocking notice. The editor keeps working when
  the hashes diverge; the operator decides whether to refresh.

## When to halt

Halt and surface in the handover (do not power through) when:

- A required Rust-side verb is missing (no analytics-template
  list, no schema-hash verb, no tenant on the session shape).
  Resolution is a Rust-side job — out of scope here.
- A `$ref`-typed IR leaf in §B3 has no obvious catalogue source
  (e.g. a unit-symbol ref with no enum and no verb that lists
  units). Resolution is a curation table addition, which is a
  design call worth surfacing.
- Stage 4's hash divergence reveals a real drift in the
  committed schema vs the agent's emit. Resolution is to fix
  the drift in the IR crate — a Rust-side job, out of scope.
- Stage 5's SSE event shape does not include a `page_id` field
  the banner can filter on. Resolution is on the rubix-agent
  side — out of scope.
- The Puck 0.19 → 0.20 (or any other version) drift surfaces a
  breaking API change. Resolution depends on the change; do not
  hide it under a workaround.
- A stage's `verify:` block fails for a reason that is not in
  the stage's own diff (e.g. an unrelated test in a sibling
  package starts failing). Fix the cause if it is one-line; halt
  if it is not.

Never `--force`, never `--no-verify`, never paper over with a
one-line "stage N.5" commit. Re-open the stage and fix inside it.
