# Workflow — mobile-chassis

This is the runbook for the `mobile-chassis` job. Read it
fully before starting phase 0, and re-read the per-phase
section before starting each phase. The scope (what to build)
lives in [SCOPE.md](./SCOPE.md). This file is how.

## Sequencing

Strict order, no batching across phases:

```
phase 0 (sdui-react ./headless split)
   ↓
phase 1 (starter-ui-core subpath fixups)
   ↓
REVIEW gate 1 — web behaviour unchanged
   ↓
phase 2 (starter-theme-tokens + kit CSS generation)
   ↓
REVIEW gate 2 — manual visual review
   ↓
phase 3 (starter-ui-kit-native — 13 primitives)
   ↓
phase 4 (starter-ui-sdui-native — 16 renderers)   [phase 5 *may* parallelise but DO NOT batch — separate commit, separate review]
   ↓
phase 5 (starter-ui-dashboard-native — 4 widgets)
   ↓
REVIEW gate 3 — final workspace sweep + handover
```

One commit per phase. One commit per REVIEW gate (a no-code
commit that records the transcript and the sign-off). Nine
commits total on `codeless/mobile-chassis`. Phase 4 and phase
5 are dependency-independent of each other (both depend on
phase 3); they MAY land in either order but must NOT be folded
into a single commit. Reviewer is entitled to review them
independently.

## Per-stage discipline

### Before writing code in any phase

1. Re-read `SCOPE.md` for the current phase (what is in, what
   is explicitly out).
2. Re-read the authoritative spec for that phase:
   - phase 0: `rubix/docs/scope/mobile/NEW-PACKAGES.md §Precondition`.
   - phase 1: `rubix/docs/scope/mobile/REUSE.md §Web fixups required`.
   - phase 2: `rubix/docs/scope/mobile/NEW-PACKAGES.md §starter-theme-tokens`.
   - phase 3: `rubix/docs/scope/mobile/NEW-PACKAGES.md §starter-ui-kit-native`.
   - phase 4: `rubix/docs/scope/mobile/NEW-PACKAGES.md §starter-ui-sdui-native` +
     `… §Parity vs the IR Kind union`.
   - phase 5: `rubix/docs/scope/mobile/NEW-PACKAGES.md §starter-ui-dashboard-native`.
3. Grep the verified-current-state references in `SCOPE.md`
   to confirm the on-disk state hasn't moved since the scope
   was written (e.g. the registry is still at
   `packages/starter-ui-sdui-react/src/renderer/registry.ts`).
   If it HAS moved, halt and update SCOPE.md first.
4. Sketch the file list for the phase (≤400 lines per file,
   one verb per file, no `utils.ts`/`helpers.ts`).
5. For phases 0 and 1 only: take a baseline transcript of
   `pnpm -w build && pnpm -w test` from the branch's starting
   point. This is what gate 1 compares against.

### Before committing any phase

1. `pnpm -w build` — clean (no new warnings, no new errors).
2. `pnpm -w test` — green across the touched packages AND
   `rubix/frontend`.
3. `pnpm -w lint` — green; do NOT add `// eslint-disable-next-line`
   on workspace import boundaries. If lint complains about a
   cross-package import, the import is wrong.
4. `pnpm -w typecheck` — green; do NOT add `// @ts-ignore` or
   `// @ts-expect-error` on workspace import boundaries.
5. For phases that touch existing packages (0, 1, 2): run the
   phase-specific regression check:
   - phase 0: the no-DOM build assertion (
     `import '@nube/starter-ui-sdui-react/headless'` MUST NOT
     reach `@nube/starter-ui-kit`).
   - phase 1: confirm `git diff master --
     packages/starter-ui-core/src/preferences/provider.tsx`
     preserves the L218/L224 `document.documentElement.lang/.dir`
     writes inside the React component half of the split.
   - phase 2: the byte-identical regression fixture (`diff
     packages/starter-ui-kit/src/styles/globals.css
     packages/starter-ui-kit/src/styles/globals.expected.css`
     or equivalent) MUST be empty.
6. For phases that create new packages (2, 3, 4, 5): confirm
   the dep graph matches `NEW-PACKAGES.md §Dependency arrow`.
   Run `pnpm --filter @nube/<new-package> exec pnpm list` and
   verify: no `react-dom`, no `starter-ui-kit` (for the
   `*-native` packages), and the expected workspace deps are
   present.
7. For phase 3 only: confirm every primitive's vitest asserts
   `accessibilityRole` + `accessibilityLabel` are forwarded.
   A primitive without that assertion is incomplete.
8. For phase 4 only: confirm the 10 deferred IR kinds
   (`stack, card, text, heading, badge, kpi_grid, button,
   link, field, sparkline`) are NOT in the side-effect
   registration list and ARE listed in the package README.
9. Re-read the staged diff. If anything is unrelated to the
   phase's scope, revert it.
10. Closing trio (below) — run all three, paste into the
    commit body or PR description.

### Closing trio (every commit, verbatim)

```bash
# checks — repeat from "before committing" so the commit body
# pins the green state at commit time
pnpm -w build
pnpm -w test
pnpm -w lint
pnpm -w typecheck

# docs — update the phase entry in CHANGELOG-style if the repo
# has one for these packages; otherwise update the per-package
# README. NEW-PACKAGES.md does NOT get updated unless the spec
# changes (it is authoritative, not a log).
ls -la packages/starter-theme-tokens/ packages/starter-ui-kit-native/ packages/starter-ui-sdui-native/ packages/starter-ui-dashboard-native/ 2>/dev/null

# git — show what landed
git status
git diff --stat HEAD
git log --oneline -5
```

No `--no-verify`. No `--force`. If a pre-commit hook fails,
fix the cause.

## REVIEW gate transcript requirements

Each REVIEW gate is its own commit on the branch (no code
changes; just `REVIEW-gate-N.md` under `.codeless/jobs/mobile-chassis/`
or the PR description, whichever the runner prefers). Reviewer
sign-off is captured in the PR thread.

### Gate 1 (after phase 1) — web behaviour unchanged

Transcript MUST include:

- `pnpm -w build` output (full, paste tail).
- `pnpm -w test` output (full, paste tail; counts must match
  or exceed baseline).
- `pnpm -w lint` output.
- `pnpm -w typecheck` output.
- The no-DOM build assertion from phase 0 (output of the
  dependency-cruiser rule or the vitest that walks the module
  graph). Must show `headless` graph is free of
  `starter-ui-kit`.
- Manual smoke transcript: `pnpm --filter @nube/rubix-frontend
  dev` boots; reviewer walks login → main dashboard → theme
  toggle → an SDUI page; reviewer confirms `lang`/`dir`
  attributes still get written on `<html>` (devtools
  Elements panel screenshot OR the explicit
  `document.documentElement.lang/.dir` console check).
- `git diff master -- packages/starter-ui-kit/` — must be
  empty (kit hasn't been touched yet).
- Phase 1 WEB-ONLY.md preview (paste contents).

Reviewer either signs off or returns the diff with specific
asks. Do not advance to phase 2 without sign-off.

### Gate 2 (after phase 2) — manual visual review

Transcript MUST include:

- All four `pnpm -w {build, test, lint, typecheck}` outputs.
- The byte-identical regression fixture output (must show
  zero diff between `globals.css` generated and the committed
  reference).
- Manual visual review checklist (filled in, with reviewer
  initials per row):

  | screen | breakpoint | light/dark | palette | matches master |
  |---|---|---|---|---|
  | login | 375 | light | default | yes/no |
  | login | 375 | dark | default | yes/no |
  | login | 768 | light | default | yes/no |
  | login | 1280 | light | default | yes/no |
  | login | 1280 | dark | palette-A | yes/no |
  | login | 1280 | dark | palette-B | yes/no |
  | dashboard | 375 | light | default | yes/no |
  | dashboard | 768 | dark | default | yes/no |
  | dashboard | 1280 | light | palette-A | yes/no |
  | dashboard | 1280 | dark | palette-B | yes/no |
  | preferences | 1280 | light | default | yes/no |
  | preferences | 1280 | dark | default | yes/no |
  | flow-editor | 1280 | light | default | yes/no |
  | flow-editor | 1280 | dark | default | yes/no |

- Reviewer sign-off note (one paragraph). Any "no" row blocks
  the gate; the cause must be diagnosed before re-running.

Do not advance to phase 3 without sign-off.

### Gate 3 (final, after phase 5) — workspace sweep + handover

Transcript MUST include:

- `pnpm -w install` output (clean).
- `pnpm -w build` output.
- `pnpm -w test` output.
- `pnpm -w lint` output.
- `pnpm -w typecheck` output.
- Per-new-package isolated build:
  - `pnpm --filter @nube/starter-theme-tokens build`.
  - `pnpm --filter @nube/starter-ui-kit-native build`.
  - `pnpm --filter @nube/starter-ui-sdui-native build`.
  - `pnpm --filter @nube/starter-ui-dashboard-native build`.
- Per-new-package dep-graph audit (`pnpm --filter <pkg> exec
  pnpm list --depth 1`), showing the expected deps and
  confirming `react-dom`/`starter-ui-kit` are absent from
  the `*-native` packages.
- Disk-overview smoke test output (the vitest that loads
  `rubix/crates/rubix-flows/dashboards/disk-overview.json`
  and renders it through the registered renderers).
- Web SPA regression sweep (`pnpm --filter
  @nube/rubix-frontend build, test`; manual login →
  dashboard → SDUI page).
- **Phase ↔ test matrix** — table mapping each phase to the
  tests/assertions/fixtures that lock its behaviour:

  | phase | test/assertion | location |
  |---|---|---|
  | 0 | headless no-DOM build assertion | `packages/starter-ui-sdui-react/__tests__/headless-graph.test.ts` (or .cruiser config) |
  | 1 | matchMedia guard sanity test | `packages/starter-ui-core/src/theme-editor/__tests__/layout-preferences.guard.test.ts` |
  | 1 | preferences DOM-write coverage | existing `provider.test.tsx` updated |
  | 2 | globals.css byte-identical fixture | `packages/starter-ui-kit/__tests__/generated-css.test.ts` |
  | 3 | per-primitive a11y assertion (×13) | `packages/starter-ui-kit-native/src/__tests__/*.test.tsx` |
  | 4 | per-renderer unit test (×16) | `packages/starter-ui-sdui-native/src/__tests__/*.test.tsx` |
  | 4 | disk-overview smoke | `packages/starter-ui-sdui-native/__tests__/disk-overview.smoke.test.tsx` |
  | 5 | per-widget unit test (×4) | `packages/starter-ui-dashboard-native/src/__tests__/*.test.tsx` |

- **Unblocked follow-up jobs** list: `rubix-mobile-scaffold`
  (the Expo app under `rubix/mobile/`) is now unblocked
  pending the separate backend bearer-issuance PR. ADR 0005
  doc PR can land in parallel. Visual-diff CI harness job
  becomes worthwhile once `starter-ui-kit-native` has a
  second consumer.

## Anti-patterns

Do not do these. If the temptation arises, halt and re-read
the relevant spec section.

- **Do not swap moti / react-native-svg for Tamagui or
  gluestack-ui** as a phase-3 deviation. That is a separate
  ADR per `NEW-PACKAGES.md`. The benefit (one design system)
  does not outweigh the cost (re-derive every primitive
  contract) inside this job's scope.
- **Do not silently register the 10 deferred IR kinds** on
  mobile. Parity with the web renderer set is the rule. If
  the mobile app *needs* one of them, the web kit gets it
  first.
- **Do not skip the byte-identical CSS regression fixture**
  in phase 2. "I eyeballed it" is not acceptance. A
  six-character HSL drift in one palette tile breaks the
  whole reason for extracting tokens.
- **Do not add `// @ts-ignore`, `// @ts-expect-error`, or
  `// eslint-disable-next-line` on workspace import
  boundaries.** If the type or the lint rule complains about
  a cross-package import, the import is wrong; fix the
  import, not the message.
- **Do not test a11y props by "looks right".** Every kit-native
  primitive's vitest MUST explicitly assert that
  `accessibilityRole` and `accessibilityLabel` are forwarded
  to the RN base element. A render snapshot alone is
  insufficient.
- **Do not edit `starter-ui-kit` in phase 0 or phase 1.** Both
  phases are additive to other packages; the kit's only
  in-scope change is in phase 2 (the generation script under
  `starter-ui-kit/scripts/`, NOT the kit's React components).
  `git diff master -- packages/starter-ui-kit/src/` should
  be empty after phase 0 and phase 1.
- **Do not fold phase 4 and phase 5 into one commit.** They
  are dependency-independent of each other and reviewer is
  entitled to review them separately.
- **Do not call `pnpm publish` from this job.** Publishing
  the new packages is a release-pipeline concern; this job
  ships the source tree only.
- **Do not introduce a mobile-only fork of a shared package.**
  Per `NON-GOALS.md`: if a prop is needed, it lands on the
  web component first (separate PR, separate job).
- **Do not start scaffolding `rubix/mobile/`** inside this
  job. That is a separate job; this job's deliverable is that
  the scaffolding job becomes unblockable.
- **Do not skip the closing trio** even if "obviously
  trivial". The trio's job is to catch the non-obvious
  regression.
- **Do not run `git push --force` or `git rebase` on the
  branch after a REVIEW gate has signed off.** Sign-off
  references a SHA; rewriting history invalidates it.

## When to halt

Halt and escalate (don't push through) if any of:

- **`matchMedia` guard regression on web.** Phase 1's
  guard-verification sanity test passing on node-without-DOM
  must not break the web-with-DOM path. If the web bundle
  starts crashing in `layout-preferences.ts` after phase 1,
  the split was wrong — revert and redo.
- **`globals.css` byte-diff is non-empty** at phase 2 commit
  time. Don't "fix" by regenerating the reference — diagnose
  the drift first. The whole point of phase 2 is no visual
  change.
- **The headless module graph contains `starter-ui-kit`**
  after phase 0. The split is paper. Revert and redo.
- **One of the 10 deferred IR kinds gets accidentally
  registered** by phase 4. Parity broken — find the rogue
  side-effect import.
- **Web SPA snapshot regresses** at any gate. The web SPA's
  passing state is the invariant the whole job protects;
  losing it is a halt-and-rollback condition, not a "land it
  and fix forward" condition.
- **`pnpm -w typecheck` requires a `// @ts-ignore` on a
  workspace boundary** to pass. The boundary is wrong; either
  the import target's `package.json` `exports` is incomplete
  or the consumer is reaching for something it shouldn't.
- **Cost cap or wall-clock cap approaches 80%** without phase
  5 being in sight. Stop, report progress to the user, and
  scope down (e.g. land phases 0–3 and split 4+5 into a
  follow-up job).
- **Phase 0's no-DOM build assertion cannot be written**
  (because dependency-cruiser or the chosen alternative
  doesn't integrate cleanly with the workspace). Halt and
  ask the user before either skipping the assertion or
  switching toolchains.
- **A spec disagreement surfaces** between the scope docs and
  the implementation reality. Halt and update the spec first
  (per the SCOPE.md "the specs win" rule); do NOT code
  around it.
