# Workflow — rubix-client-ts

## Sequencing

13 stages across four phases. Strict dependency order: A (upstream uplifts in starter-client-ts) → B (rubix-agent emits OpenAPI) → C (rubix-client-ts package) → D (CI + docs + PR). Each phase ends with a REVIEW gate. Three REVIEW gates total (A, B, C). Phase D is the closing stage; no gate after it because the PR is the gate.

**Critical dependency on the goals-2-4-3 job.** Phase B.1 starts with `git fetch origin master` + `git log --oneline -20`. If the goals-2-4-3 PR has NOT merged, raise BLOCKED with a status report and halt. The OpenAPI document Phase B emits must reflect the post-goals-2-4-3 route surface — emitting before those routes exist means a near-immediate snapshot drift after they land. Operator can unblock by waiting for the merge and resuming the job.

## Per-stage discipline

### Phase A — upstream uplifts (cheap, isolated, must be perfect)

Phase A touches `@nube/starter-client-ts`. Existing consumers of that package (`test-ui-5`, `starter-ui-kit`, anything else in `packages/`) keep working — no public API regression. Discipline:

1. **Read every existing endpoint file before extracting.** `endpoints/auth.ts`, `endpoints/theme.ts`, `endpoints/openapi.ts`, `endpoints/health.ts`. The extraction must match the exact pattern these files use; getting it wrong forces a rewrite mid-phase.
2. **Test each helper in isolation before consuming it.** `csrf.ts` gets a unit test that mocks `document.cookie`. `fetch_json.ts` gets a unit test using `vi.fn()` to stub `fetch`. Same for `fetch_void.ts` and `fetch_bytes.ts`.
3. **Refactor the existing endpoints to use the helpers in the same commit that introduces them.** If A.2 introduces helpers but A.4 (some future stage) consumes them, the helpers are dead code on master between commits. Bad git bisect surface. Land helpers + their consumers in one commit.
4. **`pnpm --filter @nube/starter-client-ts typecheck && pnpm --filter @nube/starter-client-ts test`** at the end of every Phase A stage.
5. **`pnpm --filter @nube/starter-client-ts codegen` is idempotent.** A.3's generalised codegen script must produce a byte-identical output when run with the defaults. Confirm before committing.

### Phase B — rubix-agent OpenAPI emission

Phase B is Rust work. Discipline:

1. **B.1 is gated on goals-2-4-3.** If not merged, BLOCKED. No working around it (emitting an OpenAPI document missing half the routes is worse than not emitting one at all).
2. **utoipa attributes are a sea of small edits.** B.1 inventories what's missing. B.2 implements them. Don't combine — the inventory commit is the operator's chance to spot a missing route before it becomes a snapshot drift later.
3. **The `openapi.rs` module is a verb file.** ≤ 400 lines hard, target ~150. If the assembled `OpenApi` builder exceeds that, split into `openapi/info.rs`, `openapi/tags.rs`, `openapi/paths.rs`, etc. Same R1 rule as Rust elsewhere.
4. **The snapshot must be deterministic.** B.3 must verify no timestamps, no order-dependent fields, no UUIDs in the served document. If utoipa emits any, strip them in `routes/openapi_doc.rs` before serving. The drift CI in Phase D depends on byte-identical output.
5. **`cargo test -p rubix-agent --test openapi_test`** green. `./rubix/scripts/lint-doc-refs.sh` clean.

### Phase C — the rubix-client-ts package

The largest phase by file count (~25 files across the package, ~12 endpoint families with sibling tests). Discipline:

1. **Mirror `starter-client-ts` exactly.** Same `tsconfig.json` shape, same `package.json` script names, same file layout under `src/`. A future maintainer should not need to re-learn the conventions package-to-package.
2. **One verb per file (R1).** ≤ 200 lines TS hard. Each endpoint method's file matches its Rust counterpart's verb file pattern.
3. **Sibling `.test.ts` in the same commit (R6).** Never land an endpoint without its test. The fetch-mock pattern is whatever `endpoints/auth.test.ts` uses today — confirm at C.1 and reuse.
4. **CSRF header on every mutating method.** `readCsrfHeader()` from starter-client-ts (extracted in A.2). Asserted in every mutating endpoint's `.test.ts`.
5. **Error path through `RubixError.fromResponse`.** Returns the `code` field from `body.summary.code` per SCOPE OQ-4. Asserted in at least one test per endpoint family.
6. **`pnpm --filter @nube/rubix-client-ts typecheck && pnpm --filter @nube/rubix-client-ts test`** at the end of every C stage.
7. **No hand-edits to `src/generated/index.ts`** — even if a wire type is mis-named, fix it upstream (in the utoipa attribute or schema name) and re-run codegen.

### Phase D — CI + docs + PR

One stage. Three artifacts:

1. **`rubix-openapi-drift` GH Actions job** — copy the existing `openapi-drift` job's shape exactly. Reads `rubix/openapi.json`, runs `rubix/scripts/snapshot-openapi.sh`, diffs, fails on diff.
2. **Design doc + session note** — present-tense, mirroring the conventions of existing `docs/design/*/README.md` files and `docs/sessions/2026-05-24-smoke-test-pr30.md`.
3. **PR** — `gh pr create` only after operator confirmation. Title: `feat(client): rubix-client-ts + starter-client-ts uplifts`. Body summarises each phase with the commits per phase listed.

## Anti-patterns specific to this job

- **Don't skip Phase A.** Going straight to rubix-client-ts means duplicating CSRF + fetch logic. R2 violation; future maintenance burden.
- **Don't refactor existing starter-client-ts endpoints in a separate PR.** The helpers and their first consumers (the existing endpoint files) land together. Otherwise the helpers are dead code on master between commits.
- **Don't emit `rubix/openapi.json` against a partial route surface.** If goals-2-4-3 hasn't merged, BLOCKED. Period.
- **Don't hand-write wire types in `src/generated/`.** Even if codegen produces an ugly name, fix it upstream.
- **Don't add a new endpoint family in Phase C without an entry in `rubix/openapi.json`.** If a verb exists in rubix-tools but doesn't surface via a REST route, it's not in scope for this job. Document the gap in the design doc.
- **Don't expand Phase B's tag count without expanding the package's endpoint module count.** Tag-per-goal in OpenAPI → endpoint-file-per-tag in the package. Symmetry matters for codegen sanity.
- **Don't list paths with brace expansion in handovers.** `routes/{mod.rs,tools.rs}` trips the diff-verify pre-check.
- **Don't list a path under Done that the stage didn't modify.** Same trap.
- **Don't `--no-verify`, don't `--force`** (only `--force-with-lease` after explicit rebase need, with operator confirmation).

## REVIEW gate behaviour

Each REVIEW gate commits and pushes the stage(s) that led to it; the gate itself commits nothing. Write the gate's question into `handover.md` for the next stage, halt, wait for operator confirmation.

At each REVIEW gate, the handover must include:

- One-line title per commit made in the phase, with the file count touched.
- `pnpm typecheck` + `pnpm test` summary per stage.
- For Phase B's gate: `cargo test -p rubix-agent --test openapi_test` output, the served OpenAPI route count vs `grep` count of routes in `rubix-agent::routes`, the tag count vs the goal count.
- For Phase C's gate: one operator-runnable manual flow (a tiny TS file that imports `RubixClient`, calls `disk()` and `undoLast()` against a live agent, prints the result).
- Any deviation from SCOPE (e.g. an Open Question resolved differently than the default).
- Whether the upcoming phase is unblocked.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order. Do **not** rename or reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
2. `docs` — update `handover.md` for the next stage and the active session doc.
3. `git` — stage the changes, commit with the message `stage N: <one-line title from template.yaml>`, push to `codeless/rubix-client-ts`.

A stage is not done until all three are green and the push succeeds. Never `--force`, never `--no-verify`. REVIEW gate stages mark `git` as `skipped — gate-only`. The B.1 stage marks `git` as `skipped — analysis only` if no handlers needed annotation.

## Hard rules (repeated)

- One verb per file. ≤ 400 lines Rust, ≤ 200 lines TS. No utils.ts / helpers.ts / common.ts.
- Code comments link `docs/design/<area>/README.md` only.
- No phasing markers in code.
- Upstream-first (R2). starter-client-ts changes land before rubix-client-ts consumes them.
- Wire types come from codegen. Hand-edits to `src/generated/` forbidden.
- Tests live with the code in the same commit (R6).
- Comments explain *why*, not *what*. No emojis.

## References

- `packages/starter-client-ts/` — the exemplar.
- `rubix/crates/rubix-client/` — the Rust analogue.
- `crates/starter-server/src/openapi.rs`, `crates/starter-server/src/routes/openapi_doc.rs` — the OpenAPI emission pattern to mirror.
- `rubix/crates/rubix-spi/src/dto/` — DTOs with utoipa `ToSchema` attached.
- `rubix/SCOPE.md` — R1–R13.
- `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
- `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md`.
- `.github/workflows/openapi-drift.yml` — the CI pattern to mirror in Phase D.
