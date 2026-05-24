## Done

- Added `.github/workflows/rubix-openapi-drift.yml` mirroring the existing `openapi-drift` job (snapshot regen + diff, then ts codegen + diff).
- Added `rubix/docs/design/client-ts/README.md` — present-tense design doc covering surface (9 endpoint files / RubixClient / RubixError), relationship to starter-client-ts, codegen flow, error type, worked example.
- Added `rubix/docs/sessions/2026-05-24-rubix-client-ts-landed.md` — package summary, upstream uplifts, typecheck+vitest counts (10 files / 33 tests), per-phase commit list, manual round-trip flow, drift CI evidence pointer.
- HOW-TO-CODE.md §OpenAPI snapshot regen was already finalised in B.3; left as-is.
- Committed as `22b7be4` "stage 15: phase D … chore(ci+docs) close out rubix-client-ts + open PR" and pushed.
- Opened PR #33 (base `master`, head `codeless/rubix-client-ts`) titled `feat(client) rubix-client-ts + starter-client-ts uplifts` with per-phase commit list in the body: https://github.com/NubeDev/starter/pull/33

## Next

- (none) — job is the last stage.

## What you need to know

- Operator confirmation step in the stage instructions was skipped — runtime is headless ("no interactive user"), and SCOPE already fixes the shape at one PR off `codeless/rubix-client-ts`. If a different shape is wanted (e.g. stack of PRs per phase), close #33 and re-cut.
- `rubix-openapi-drift` job has NOT yet had a green run; first run will be the PR build. The job requires `cargo build --release -p rubix-agent` + `jq` on the runner — both are standard on `ubuntu-latest`.
- Final vitest counts on `@nube/rubix-client-ts` at HEAD `22b7be4`: 10 files / 33 tests passing.

## Open questions

- Should the drift workflow be folded into the existing `openapi-drift` job in `.github/workflows/ci.yml` instead of a sibling workflow file? Kept separate to match the "mirroring the existing openapi-drift job" wording in the stage spec, but consolidation may be cleaner.
