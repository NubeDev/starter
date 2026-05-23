# Workflow — rubix-phase-0

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md` and the authoritative source SCOPE at
[/home/user/code/rust/starter/rubix/SCOPE.md](/home/user/code/rust/starter/rubix/SCOPE.md).

## Sequencing

Three stages, two REVIEW gates between them. Strictly linear:

- Stage 2 (testcontainer smokes) cannot start until stage 1's
  crate skeletons exist and the workspace builds — the smoke
  tests live inside `data-postgres` / `data-clickhouse` crates
  that stage 1 creates.
- Stage 3 (design docs) builds on top of the actual workspace
  shape stage 1 lands. `OVERVIEW.md`'s dependency arrows must
  match the real `Cargo.toml` and `pnpm-workspace.yaml`; if stage
  3 starts before stage 1 freezes the layout, the docs drift
  immediately.

The two REVIEW gates exist because:
- Gate 1 (after stage 1): the workspace shape is load-bearing
  for the whole repo. A wrong skeleton at this point ripples
  through every later phase. Catch it now.
- Gate 2 (after stage 3): `AUTH.md` and `MIGRATIONS.md`
  completeness is the Phase 1 entry gate. If they're
  incomplete, Phase 1 starts against a fog and rots.

## Per-stage discipline

Before writing any code or docs in a stage:

1. Re-read the corresponding section of the source SCOPE. The
   SCOPE text is the contract; this WORKFLOW is the process.
2. Re-read `SCOPE.md` §"In scope" and §"Out of scope". The
   biggest risk on this job is creeping into Phase 1 territory
   — implementing a kind, writing a domain function,
   sketching a Studio page. Stay strictly within Phase 0.
3. For stage 1: enumerate the existing `starter/Cargo.toml`
   workspace members before editing — confirm the rubix crates
   slot in alphabetically under a `# rubix tree` comment and
   nothing in the existing list shifts.
4. For stage 3: re-read each rule (R1-R13) before writing the
   doc that expands on it. The doc's job is *cite and
   elaborate*, not *invent new rules*.

Before committing a stage:

1. `cargo build --workspace` green from the starter repo root.
2. `cargo clippy --workspace --all-features -- -D warnings`
   green.
3. `cargo fmt --check` green.
4. `mani run build --all` green; `mani run lint` green (and for
   stage 1 specifically, demonstrably catches a synthetic
   401-line violator that's added then removed within the
   stage).
5. For stage 2 specifically: the testcontainer smokes pass
   against a local docker (record the docker version + the
   `cargo test -p ... -- --ignored` transcript in the
   handover).
6. For stage 3 specifically: every doc is under 400 lines (run
   `wc -l rubix/docs/design/*.md` and capture the output in
   the handover), every doc cites at least one source-SCOPE
   rule number, and `AUTH.md` + `MIGRATIONS.md` both contain
   enough concrete content that a Phase 1 contributor could
   start against them.

Commit + push via **mani** from the codeless-workspace root:

```
./bin/mani --config mani.yaml run commit --projects starter \
  MSG='stage N: <one-line title from template.yaml>'
./bin/mani --config mani.yaml run push --projects starter
```

No `--force`, no `--no-verify`.

## Closing trio — the last three todos of every stage

1. `checks` — the build/lint/fmt/mani run lint matrix above.
   Stage 2 additionally runs the testcontainer smokes; stage 3
   additionally runs the doc-length and citation checks.
2. `docs` — update `handover.md` for the next stage (or for the
   exit summary at stage 3), tick the relevant `[x]` in
   `SCOPE.md` §"Deliverables".
3. `git` — stage the changes, commit with `stage N: <title>`,
   push to `codeless/rubix-phase-0`. One stage, one commit.

A stage is not "done" until all three are green and the push
succeeds.

## REVIEW gates

Two gates: after stage 1 and after stage 3.

**Gate 1 (after stage 1)** — write a handover comment containing:
- The `cargo build --workspace` transcript proving every new
  skeleton crate compiles.
- The `pnpm install` transcript proving the TS workspace
  resolves and cross-package links work.
- The `mani run lint` transcript on a synthetic 401-line file
  proving the R1 enforcement actually works (then remove the
  violator).
- The diff of `starter/Cargo.toml`'s `members` array showing the
  rubix crates added alphabetically under a `# rubix tree`
  comment.

**Gate 2 (after stage 3)** — write a handover containing:
- `wc -l rubix/docs/design/*.md` output proving every doc is
  under 400 lines.
- A `grep -c 'R[0-9]\+\|R1-R13' rubix/docs/design/*.md` table
  proving every doc cites at least one source-SCOPE rule.
- A two-paragraph excerpt from `AUTH.md` and one from
  `MIGRATIONS.md` demonstrating they contain concrete content,
  not just an outline.
- A brief Phase 1 readiness assessment: "could a contributor
  write `domain-devices` against these eight docs alone?" with
  a yes/no answer and the reasoning.

Do not declare the job done without explicit approval at gate 2.

## Anti-patterns specific to this job

- **Do not** write a single line of domain logic. Phase 0 is
  structure + docs. The temptation is to "sketch a kind manifest
  while I'm at it" or "leave a placeholder NodeBehavior impl";
  resist. Every line that isn't structure or docs is Phase 1
  scope creep.
- **Do not** create a second Cargo workspace under `rubix/`. R0
  binds: rubix consumes starter from the same workspace. A
  parallel workspace is a load-bearing wrong turn.
- **Do not** put `// TODO Phase 1` comments in skeleton files.
  R12 binds: no session-progress chatter. An empty lib.rs is
  fine; an empty lib.rs decorated with `// TODO: implement
  devices kind` is a R12 violation.
- **Do not** widen `rubix-spi` beyond `starter-spi` re-exports.
  R5 binds: zero internal deps. The Phase 0 skeleton declares
  module slots (empty `pub mod`s), nothing else.
- **Do not** start the design docs with an "Overview" preamble
  that restates the source SCOPE in different words. Each doc
  picks the source-SCOPE rule it owns and elaborates with
  concrete examples + a worked code-shape sample where
  appropriate. Cite and elaborate, do not paraphrase.
- **Do not** write `RUNTIME.md`, `ARTIFACTS.md`, `BACKUP.md`,
  `QUERY-LANG.md`, `EXTENSIONS.md`, `LOGGING.md`, `UI.md`,
  `MCP.md`, `NODE-RED-MODEL.md`, `HOW-TO-ADD-CODE.md`, or
  `SDUI.md` in this job. The source SCOPE explicitly defers
  these to the phases that need them. Writing them early ages
  them; let each phase author them just-in-time.
- **Do not** ship the MQTT reference extension. That's Phase 3.
  An empty `extensions/` directory in stage 1 is the maximum
  this job allows.
- **Do not** wire actual REST/gRPC/MCP routes in
  `transport-rest` / `-grpc` / `-mcp`. The Phase 0 source SCOPE
  doesn't even list these as skeleton crates; they land in
  Phase 1 when the first domain crate needs to be served.
- **Do not** make the `mani run lint` 400-line check skip any
  file pattern. If a generated file legitimately exceeds 400
  lines (e.g. a codegen output), the right answer is to add the
  generator path to the workspace's existing
  per-language-tool ignore rules, not to weaken the lint task.
- **Do not** treat any `[!]` mark as recoverable inside this
  job. R1's load-bearing-ness is precisely what makes the
  Phase 0 layout permanent; a half-built workspace is worse
  than no workspace.

## When to halt

- The `starter/Cargo.toml` workspace cannot cleanly absorb the
  rubix crates (e.g. a feature conflict surfaces between
  `starter-store-postgres`'s default features and what
  `rubix-data-postgres` needs). Halt at stage 1; the resolution
  is to factor the feature on the starter side, which is
  starter-scope, not rubix-scope. Surface in chat.
- The testcontainer smoke at stage 2 cannot pass because the
  existing starter `testing` seam doesn't accept a no-schema
  caller. Surface; the resolution is in the seam (starter-side),
  not in rubix.
- The `mani run lint` 400-line check turns out to be
  prohibitively slow on the full tree at scale (unlikely at
  Phase 0 scale, but possible). Surface; the resolution is to
  switch to a faster file walker, not to weaken the rule.
- A design doc's content exceeds 400 lines and cannot be
  honestly compressed below that limit. Halt and surface; the
  resolution is to split the doc (e.g. `AUTH.md` →
  `AUTH-SESSIONS.md` + `AUTH-AUTHZ.md`), not to relax R1.
- Phase 1 work starts to bleed in. Halt immediately. R12 + the
  Phase 0 scope are non-negotiable.
