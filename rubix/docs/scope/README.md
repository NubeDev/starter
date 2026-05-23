# docs/scope/

**Tier:** Plans, not the system. Lifetime: weeks to months.

Per [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md):

- What we plan to build and why.
- Evolves with the product. Items leave when they land in
  `docs/design/` or are explicitly dropped.
- **Never** referenced from source code.

## Contents

- [GAPS.md](./GAPS.md) — the rolling audit of starter capabilities
  rubix has not yet accounted for. Phase reviewers consult this
  at every entry / exit gate.
- [THIN-SLICE.md](./THIN-SLICE.md) — the active plan: one demo
  path that exercises every architectural layer end-to-end, in
  five PRs.

## Active codeless jobs

When a plan above is being executed by a codeless agent, the job
manifest lives outside this tree under `.codeless/jobs/<name>/`:

- `rubix-thin-slice` — executes THIN-SLICE.md blocks 1–5. See
  [`/home/user/code/rust/starter/.codeless/jobs/rubix-thin-slice/`](/home/user/code/rust/starter/.codeless/jobs/rubix-thin-slice/).
  Three files: `SCOPE.md` (per-job scope), `template.yaml` (the
  executable stage list), `WORKFLOW.md` (the human driving
  process).

When a gap promotes, the row in GAPS.md flips to a back-reference
("addressed in phase N — see docs/design/<area>/") and the new
design doc carries the canonical description forward.
