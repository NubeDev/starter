# Scope — mobile-chassis

The authoritative design lives in two scope documents and one
ADR on master:

- [/home/user/code/rust/starter/rubix/docs/adr/0004-react-native-mobile-app.md](/home/user/code/rust/starter/rubix/docs/adr/0004-react-native-mobile-app.md)
  — the decision to ship a React Native (Expo SDK 52+, locked
  in per §D) mobile app, and the four-package split as a
  precondition.
- [/home/user/code/rust/starter/rubix/docs/scope/mobile/NEW-PACKAGES.md](/home/user/code/rust/starter/rubix/docs/scope/mobile/NEW-PACKAGES.md)
  — the four new workspace packages, the `./headless` split
  precondition with three concrete refactors, and the parity-vs-IR
  enumeration.
- [/home/user/code/rust/starter/rubix/docs/scope/mobile/REUSE.md](/home/user/code/rust/starter/rubix/docs/scope/mobile/REUSE.md)
  — the named-export reuse matrix, the `theme-editor` forbid list,
  and the §"Web fixups required" list (three upstream changes).
- [/home/user/code/rust/starter/rubix/docs/scope/mobile/APP-SHELL.md](/home/user/code/rust/starter/rubix/docs/scope/mobile/APP-SHELL.md)
  — context only (provider stack, import-lint). This job does
  NOT build the Expo app; APP-SHELL is what unlocks after.

This brief is the per-job scope. **Where this disagrees with the
specs, the specs win** — fix this file rather than diverge. The
specs have been through two peer-review passes; the design is
settled.

## Goal

Land the workspace-level chassis the mobile plan depends on, on
the `codeless/mobile-chassis` branch of the `starter` repo. After
this job:

1. `@nube/starter-ui-sdui-react` exposes a `./headless` subpath
   (registry + provider + hooks + transport + `Render`), and
   `src/sdui-page.tsx` no longer transitively pulls every web
   renderer through the barrel.
2. `@nube/starter-ui-core` is shaped so the named exports listed
   in `REUSE.md` are reachable from RN consumers without
   dragging DOM-bound siblings (`PreferencesProvider` split out;
   `theme-editor` web-only surface documented).
3. `@nube/starter-theme-tokens` exists as pure-data source of
   truth; `starter-ui-kit/src/styles/globals.css` is generated
   from it and is byte-identical to today.
4. `@nube/starter-ui-kit-native` exists with 13 primitives, RN
   prop API mirroring the web kit, every primitive accessible.
5. `@nube/starter-ui-sdui-native` exists with 16 renderers
   matching the web set one-for-one, registered against the
   shared `./headless` registry.
6. `@nube/starter-ui-dashboard-native` exists with RN ports of
   the four dashboard widgets.
7. The web SPA (`rubix/frontend`) builds, tests, lints, and
   boots unchanged across every phase boundary — verified at
   gate 1, gate 2, and gate 3.
8. The `rubix/mobile/` scaffolding job is now unblocked. That
   job is **out of scope here** (it gets its own job; this job
   merging is the only precondition).

## In scope (six phases)

### Phase 0 (stage 1) — `starter-ui-sdui-react` `./headless` split

- Move `src/renderer/registry.ts` to `src/headless/registry.ts`;
  update every `render-*.tsx` import. Verified-current-state:
  registry is at `packages/starter-ui-sdui-react/src/renderer/registry.ts`
  today.
- Decouple `src/sdui-page.tsx` from the renderer barrel. Today
  (verified) it does
  `import { Render, listRenderers } from "./renderer/index.js";`
  — the barrel that side-effect-registers every web renderer.
  Move `Render` + the page logic into `src/headless/` so
  `sdui-page.tsx` imports `./headless/render.js` +
  `./headless/registry.js` only.
- Move `SduiProvider`, `src/hooks/*`, the transport, and `Render`
  into `src/headless/`.
- Add `"./headless": { types, import }` to
  `packages/starter-ui-sdui-react/package.json` `exports`.
- Update `src/index.ts` (root) to re-export from
  `./headless/index.js` for backwards-compat and continue to
  side-effect-register the web renderers (`./renderer/index.js`)
  for existing web consumers.
- Add a build/unit assertion (`dependency-cruiser` rule or
  vitest that walks the module graph) proving that importing
  `@nube/starter-ui-sdui-react/headless` does NOT reach
  `@nube/starter-ui-kit`. Without this assertion the split is
  paper.
- Acceptance: `pnpm -w build` clean; web SPA boots; the assertion
  passes; existing root-import consumers (the web SPA itself)
  see no behaviour change.

### Phase 1 (stage 2) — `starter-ui-core` subpath fixups

- Split `src/preferences/provider.tsx` (today writes
  `document.documentElement.lang` at L218 and `.dir` at L224 —
  verified) into:
  - a pure store + types + formatters module (DOM-free);
  - a `PreferencesProvider` React component that keeps the
    web-side document-writing behaviour identical.
- Audit `src/theme-editor/`. Keep the following web-only
  helpers exactly as-is, and document their web-only status
  in `packages/starter-ui-core/src/theme-editor/WEB-ONLY.md`
  listing each by name:
  - `localStorageThemeTransport` from
    `theme-editor/transport.ts` (uses `window.localStorage` at
    L96, L106 — verified).
  - `utils/apply-theme.ts`, `utils/apply-preferences.ts`,
    `utils/generate-css.ts`, `utils/tailwind-css.ts`,
    `utils/parse-css-input.ts` (write to
    `document.documentElement` or emit CSS strings).
- Ensure `types`, `defaults`, `presets`, the editor `store`, the
  `layout-preferences` store + types, `utils/color-converter`,
  `utils/contrast-checker` are reachable as named exports from
  `./theme-editor`.
- Verify (sanity test) the `layout-preferences.ts:81,92,104`
  `matchMedia` guards
  (`typeof window === "undefined" || typeof window.matchMedia !== "function"`)
  no-op cleanly under a jsdom-disabled / node-only environment.
  This is the same code path Hermes hits.
- Additive only: NO new packages; NO new `exports` entries
  unless required to make a named re-export reachable.
  `starter-ui-core/auth` and `starter-ui-core/query` are not
  touched.

### Phase 2 (stage 3) — `starter-theme-tokens` new package

- New `packages/starter-theme-tokens/`. Pure data. No React, no
  RN, no DOM, no styling runtime. ESM + CJS dual build per the
  workspace convention.
- One file per concept per `NEW-PACKAGES.md §starter-theme-tokens`:
  - `src/palette.ts` — HSL triplets, named palettes extracted
    from `starter-ui-kit/src/styles/globals.css` and
    `starter-ui-core/src/theme-editor/presets.ts`.
  - `src/density.ts`, `src/radius.ts`, `src/type.ts`,
    `src/motion.ts`, `src/role.ts`.
  - `src/index.ts` — barrel, re-exports only.
- `starter-ui-kit/scripts/generate-css.ts` reads tokens and
  emits `src/styles/globals.css` at build time. The emitted
  file MUST be byte-identical to today's hand-written file;
  commit a regression fixture that diffs them.
- `starter-ui-core/src/theme-editor/presets.ts` re-exports from
  the tokens package — delete the hardcoded palette tables.
- Add `starter-theme-tokens` to root `pnpm-workspace.yaml`.

### Phase 3 (stage 4) — `starter-ui-kit-native` new package

- New `packages/starter-ui-kit-native/`. peerDependencies on
  `react`, `react-native`, `react-native-svg`, `moti`,
  `react-native-reanimated`. Depends on
  `@nube/starter-theme-tokens` + `@nube/starter-ui-core/theme-editor`
  (named exports only — layout-preferences store + types).
- 13 primitives, one file per primitive, each ≤200 lines, per
  `NEW-PACKAGES.md §starter-ui-kit-native`:
  `button`, `card`, `input`, `tabs`, `badge`, `switch`,
  `slider`, `select`, `sheet`, `dialog`, `spinner`, `skeleton`,
  `tooltip`.
- Prop API mirrors `starter-ui-kit` one-to-one. `onClick` on
  web → `onPress` on RN; everything else identical.
- **Every primitive ships `accessibilityRole` +
  `accessibilityLabel` / `accessibilityHint` props wired through
  to the RN base element.** This is a kit acceptance criterion,
  NOT a polish item. A primitive PR without it is rejected.
- `useTheme()` hook reads from `starter-theme-tokens` + the
  layout-preferences store. No `className`, no
  `StyleSheet.create()` calls outside the component file that
  uses them.
- Story-style harness under `packages/starter-ui-kit-native/example/`
  rendering each primitive in isolation against light / dark /
  two named palettes.
- Vitest unit tests per primitive (snapshot + a11y prop
  assertion).
- MUST NOT import `starter-ui-kit`, MUST NOT do network I/O,
  MUST NOT own application state.
- Foundation: RN core + react-native-svg + moti. Tamagui /
  gluestack-ui swap is a separate ADR per the NEW-PACKAGES
  decision; do NOT adopt as a phase-3 deviation.

### Phase 4 (stage 5) — `starter-ui-sdui-native` new package

- New `packages/starter-ui-sdui-native/`. peerDependencies on
  `react`, `react-native`. Depends on
  `@nube/starter-ui-kit-native`, `@nube/starter-ui-ir`,
  `@nube/starter-ui-sdui-react/headless` (which exists as of
  phase 0).
- 16 renderer files matching
  `packages/starter-ui-sdui-react/src/renderer/index.ts`
  priority order one-for-one:
  `render-page`, `render-row`, `render-col`, `render-grid`,
  `render-kpi`, `render-chart`, `render-divider`, `render-tabs`,
  `render-table`, `render-form`, `render-select`,
  `render-slider`, `render-toggle`, `render-date-range`,
  `render-repeat`, `render-custom`.
- Each renderer ≤150 lines. Each imports ONLY
  `starter-ui-kit-native` + `starter-ui-ir` types. No direct RN
  primitives — keeps styling consistent and renderers testable
  against a swap-in mock kit.
- Single barrel `src/index.ts` that side-effect
  `registerRenderer(...)`s every kind on import.
- The 10 IR kinds the web does NOT register
  (`stack, card, text, heading, badge, kpi_grid, button, link, field, sparkline`,
  per `NEW-PACKAGES.md §Parity vs the IR Kind union`) are
  listed in the package README as deferred-with-web. NOT
  silently registered — parity with the web renderer set is
  the rule.
- Vitest unit tests per renderer using a mock kit-native module
  and IR fixtures from `starter-ui-ir`.
- Smoke fixture: the existing
  `rubix/crates/rubix-flows/dashboards/disk-overview.json`
  (uses `page, row, col, kpi, chart` + `static` slot bindings —
  verified) must render through the registered renderers in a
  unit test.

### Phase 5 (stage 6) — `starter-ui-dashboard-native` new package

- New `packages/starter-ui-dashboard-native/`. peerDependencies
  on `react`, `react-native`, `react-native-svg`. Depends on
  `@nube/starter-ui-kit-native`.
- RN ports of the four dashboard widgets in
  `packages/starter-ui-dashboard/`, identical prop APIs (a
  feature consumed on web ships on mobile by changing only the
  import).
- Vitest unit tests per widget.
- No direct RN primitives — same discipline as sdui-native
  (only kit-native + svg).

## Out of scope

- **`rubix/mobile/` Expo app scaffold.** That is a separate job
  (`rubix-mobile-scaffold` or similar). This job's deliverable
  is that the scaffolding job is unblocked.
- **Backend `POST /api/v1/auth/token` route.** A backend PR per
  `rubix/docs/scope/mobile/APP-SHELL.md §Backend prerequisite`.
  Bearer **acceptance** already works (`principal_layer.rs`
  reads `Authorization: Bearer`); the missing piece is a
  credentials→token issuance route. Not this job.
- **Per-connection SQLite local DB design.** Lives in
  `rubix/docs/scope/mobile/LOCAL-DB.md`; ships with the Expo
  app scaffold job, not here.
- **Visual-diff CI harness for the web kit** (Playwright or
  similar). Defensible default: manual review for Block 1
  (gate 2 of this job), harness in a follow-up Block 2 when
  there are two consumers to justify the infra.
- **ADR 0005** (retroactive ADR for the existing web SPA).
  Identified as a doc-tier gap in peer review #2; a separate
  doc-only PR, not this job.
- **TLS / self-signed cert handling on RN.** Non-goal per
  `rubix/docs/scope/mobile/NON-GOALS.md`; reserved schema hook
  (`tls_pinned_fingerprint`) lives with the local-db job.
- **Maestro e2e harness.** Lives with the Expo app scaffold
  job; this chassis job has no RN runtime in CI.
- **Mobile-only forks of any shared package.** Per
  `NON-GOALS.md`: if a prop is needed, it lands on the web
  component first.
- **Modifying any existing crate's or package's public API
  beyond the explicit refactors above.** This job is additive
  on `starter-ui-sdui-react` (`./headless` subpath) and
  `starter-ui-core` (`PreferencesProvider` split + WEB-ONLY
  documentation), and net-new for the four mobile packages.
  Nothing else moves.

## Constraints

- **R6 (rubix `SCOPE.md`) — the reuse seam.** The only
  DOM-bound layers are `starter-ui-kit` (web) and the renderer
  barrel under `starter-ui-sdui-react/src/renderer/`. Anything
  RN consumers reach must avoid both.
- **Rule Zero (FILE-LAYOUT.md §0) — file-size budget.** ≤400
  lines per file; ≤50 lines per function; one verb per file;
  no `utils.rs`/`helpers.rs`/`common.rs` equivalents in TS
  (`utils.ts`/`helpers.ts`/`common.ts` are also banned).
- **Web behaviour unchanged.** Every phase must leave
  `rubix/frontend` building, testing, linting, and booting
  identically to master. Visual output must not change in
  phases 0, 1, 2; phase 2 is the only one that touches the
  styling pipeline and that change must be byte-identical
  (`globals.css` generated == `globals.css` hand-written today).
- **Named-export discipline.** `REUSE.md` allows specific named
  exports from `starter-ui-core/auth`, `…/query`, `…/i18n`,
  `…/preferences` (split), `…/theme-editor`. Phase 1 documents
  the web-only siblings; the import-lint rule that enforces
  this lives in `rubix/mobile/` (NOT this job — but the
  WEB-ONLY documentation is what makes that lint rule
  authorable).
- **`./headless` is a hard precondition for phase 4.** Phase 4
  cannot start until phase 0 ships and the no-DOM build
  assertion is green. If phase 0's assertion fails, phase 4 is
  blocked — fix phase 0 before advancing.
- **Parity-with-web for sdui-native.** The 16 renderers match
  the web set one-for-one in name and priority. The 10
  unimplemented-on-web IR kinds are deferred-with-web — not
  silently shipped on mobile.
- **A11y is a kit acceptance criterion.** Per phase 3: every
  primitive ships `accessibilityRole` + `accessibilityLabel`
  wiring. A primitive PR without it is rejected at review.
- **No `--no-verify`, no `--force`.** If a pre-commit hook
  fails, fix the cause.
- **Lint / test / type gates green at every phase boundary**:
  `pnpm -w build`, `pnpm -w test`, `pnpm -w lint`,
  `pnpm -w typecheck`.

## Deliverables (what "done" looks like)

1. `codeless/mobile-chassis` branch with one commit per phase
   (six phases + three REVIEW handovers = nine commits), pushed.
2. **Phase 0 acceptance:** `pnpm --filter @nube/starter-ui-sdui-react build`
   green; the no-DOM build assertion proving
   `import '@nube/starter-ui-sdui-react/headless'` does not
   reach `@nube/starter-ui-kit` passes; `rubix/frontend` boots
   unchanged.
3. **Phase 1 acceptance:** `pnpm --filter @nube/starter-ui-core test`
   green; web `PreferencesProvider` still mounts and writes
   `document.documentElement.lang/.dir`; `theme-editor/WEB-ONLY.md`
   names every web-only helper; the matchMedia-guard sanity
   test passes.
4. **Gate 1 acceptance:** full workspace clean —
   `pnpm -w {build, test, lint, typecheck}` green;
   `rubix/frontend` manual smoke (login → dashboard → theme
   toggle → SDUI page) captured as a transcript;
   `git diff master -- packages/starter-ui-kit/` empty.
5. **Phase 2 acceptance:** `starter-theme-tokens` published in
   workspace; `globals.css` generated from tokens is
   byte-identical to today's file (regression fixture passes);
   `starter-ui-core/src/theme-editor/presets.ts` re-exports
   from tokens.
6. **Gate 2 acceptance:** reviewer signs off on the manual
   visual review (login / dashboard / preferences / one flow
   editor screen, three breakpoints, light + dark + two named
   palettes).
7. **Phase 3 acceptance:** `pnpm --filter @nube/starter-ui-kit-native build, test`
   green; 13 primitives, each ≤200 lines, each with a11y props
   and a vitest assertion that the a11y props are wired;
   example harness renders.
8. **Phase 4 acceptance:** `pnpm --filter @nube/starter-ui-sdui-native build, test`
   green; 16 renderers; the `disk-overview.json` smoke renders;
   the 10 deferred IR kinds named in the package README.
9. **Phase 5 acceptance:**
   `pnpm --filter @nube/starter-ui-dashboard-native build, test`
   green; four widgets, identical prop API to the web kit.
10. **Gate 3 acceptance (final):** full workspace sweep —
    `pnpm -w {install, build, test, lint, typecheck}` green;
    each new package's dep graph contains no `react-dom` and
    no `starter-ui-kit`; web SPA regression sweep clean;
    final handover ships the phase ↔ test matrix and the
    unblocked-follow-up-job list.

## Open questions — RESOLVED (2026-05-25, before start)

### Q1 — One job or split per package?

**Answer: one job, six phases, three REVIEW gates.**

The four new packages are tightly coupled
(`starter-ui-kit-native` depends on `starter-theme-tokens`;
`starter-ui-sdui-native` depends on `starter-ui-kit-native` +
`starter-ui-sdui-react/headless`; `starter-ui-dashboard-native`
depends on `starter-ui-kit-native`), and they depend on two
upstream refactors. Splitting per package would land partial,
non-useful intermediate states. The three REVIEW gates catch:

- **After phases 0+1:** silent regression in the web SPA from
  the upstream refactors. Caught before any net-new package
  exists.
- **After phase 2:** visual drift in the web kit from the
  CSS-generation refactor. Manual review, sign-off in the PR.
- **After phases 3+4+5:** every new package builds in
  isolation and as part of the workspace; deps are correct;
  the web SPA is still healthy.

Cap: **45000¢ / 6h** — six phases of TS scaffolding and
mechanical extraction. No long test cycles (no DB, no CH, no
RN simulator). Phase 0 small (~10%), phase 1 small (~10%),
phase 2 small-medium (~15%, the byte-identical regression
fixture is the hard part), phase 3 the bulk (~30%), phase 4
medium (~20%), phase 5 small (~10%), gates + sweep (~5%).

### Q2 — Implementation order within phase 3 (kit-native)?

**Answer: container/layout primitives first, then interaction, then overlays.**

Order:
1. `card`, `badge`, `spinner`, `skeleton` (pure presentation;
   no state, no a11y interaction surface — establishes the
   `useTheme()` pattern and the file-shape).
2. `input`, `switch`, `slider`, `select` (form controls; each
   has a real a11y contract).
3. `tabs` (composed presentation + a11y).
4. `button` (looks trivial but every other primitive's a11y
   pattern leans on the Pressable wiring established here).
5. `sheet`, `dialog`, `tooltip` (overlays; depend on portal
   semantics that may interact with reanimated).

Each primitive gets its own commit-suitable unit (do not
batch). The story-harness page for each lands in the same
commit as the primitive — story without primitive or primitive
without story is incomplete.

### Q3 — `./headless` registry: shared module instance how?

**Answer: re-export, not duplicate.**

The web renderer set must continue to call the same
`registerRenderer` the `./headless` consumers call. The
registry MOVES to `src/headless/registry.ts`; both the web
renderer barrel and the future mobile renderer barrel import
the same module. No second copy. This is verified by phase 0's
no-DOM build assertion: it walks the headless module graph
and asserts the registry it finds is the SAME module that
`packages/starter-ui-sdui-react/src/renderer/render-page.tsx`
imports.

If pnpm's symlink layout ever produces two copies (it
shouldn't, but it has burned us before), the symptom is
silent renderer de-registration; the assertion catches it.

### Q4 — Visual-diff CI harness now or later?

**Answer: later. Manual review is the phase 2 gate; harness
is a follow-up job.**

Per the user's defensible default. Justification: there are
zero consumers of the visual harness today; adding one for a
single refactor whose acceptance bar is "byte-identical CSS
output" is over-engineering. The byte-identical regression
fixture in phase 2 covers the source-of-truth invariant;
manual review covers the rendering invariant (since
byte-identical CSS == identical rendering, this is belt +
braces). Once `starter-ui-kit-native` lands and the web
visual contract has two consumers, a harness pays for itself.

### Q5 — What unblocks `rubix/mobile/` scaffolding?

**Answer: this job merging + the backend bearer-issuance route.**

This job is the chassis precondition. The other precondition
(backend `POST /api/v1/auth/token` route) is a separate
backend PR, called out in
`rubix/docs/scope/mobile/THIN-SLICE.md §Pre-Block 4`. The
two are independent — they can land in either order. The
`rubix/mobile/` scaffolding job blocks on both.

## References

- ADR (authoritative): [/home/user/code/rust/starter/rubix/docs/adr/0004-react-native-mobile-app.md](/home/user/code/rust/starter/rubix/docs/adr/0004-react-native-mobile-app.md)
- NEW-PACKAGES (authoritative for phases 2–5): [/home/user/code/rust/starter/rubix/docs/scope/mobile/NEW-PACKAGES.md](/home/user/code/rust/starter/rubix/docs/scope/mobile/NEW-PACKAGES.md)
- REUSE (authoritative for phases 0–1 + the named-export discipline): [/home/user/code/rust/starter/rubix/docs/scope/mobile/REUSE.md](/home/user/code/rust/starter/rubix/docs/scope/mobile/REUSE.md)
- APP-SHELL (context only, not implemented here): [/home/user/code/rust/starter/rubix/docs/scope/mobile/APP-SHELL.md](/home/user/code/rust/starter/rubix/docs/scope/mobile/APP-SHELL.md)
- LOCAL-DB (context only, not implemented here): [/home/user/code/rust/starter/rubix/docs/scope/mobile/LOCAL-DB.md](/home/user/code/rust/starter/rubix/docs/scope/mobile/LOCAL-DB.md)
- NON-GOALS (mobile-wide, applies here too): [/home/user/code/rust/starter/rubix/docs/scope/mobile/NON-GOALS.md](/home/user/code/rust/starter/rubix/docs/scope/mobile/NON-GOALS.md)
- THIN-SLICE (downstream consumer; this job unblocks it): [/home/user/code/rust/starter/rubix/docs/scope/mobile/THIN-SLICE.md](/home/user/code/rust/starter/rubix/docs/scope/mobile/THIN-SLICE.md)
- Existing sdui-react root barrel (phase 0 target): [/home/user/code/rust/starter/packages/starter-ui-sdui-react/src/index.ts](/home/user/code/rust/starter/packages/starter-ui-sdui-react/src/index.ts)
- Existing sdui-page (phase 0 decoupling target): [/home/user/code/rust/starter/packages/starter-ui-sdui-react/src/sdui-page.tsx](/home/user/code/rust/starter/packages/starter-ui-sdui-react/src/sdui-page.tsx)
- Existing renderer index (phase 0 + phase 4 priority-order source): [/home/user/code/rust/starter/packages/starter-ui-sdui-react/src/renderer/index.ts](/home/user/code/rust/starter/packages/starter-ui-sdui-react/src/renderer/index.ts)
- Existing preferences provider (phase 1 split target): [/home/user/code/rust/starter/packages/starter-ui-core/src/preferences/provider.tsx](/home/user/code/rust/starter/packages/starter-ui-core/src/preferences/provider.tsx)
- Existing theme-editor transport (phase 1 WEB-ONLY documentation target): [/home/user/code/rust/starter/packages/starter-ui-core/src/theme-editor/transport.ts](/home/user/code/rust/starter/packages/starter-ui-core/src/theme-editor/transport.ts)
- Existing theme-editor layout-preferences (phase 1 guard-verification target): [/home/user/code/rust/starter/packages/starter-ui-core/src/theme-editor/layout-preferences.ts](/home/user/code/rust/starter/packages/starter-ui-core/src/theme-editor/layout-preferences.ts)
- Existing kit globals (phase 2 source data + acceptance bar): [/home/user/code/rust/starter/packages/starter-ui-kit/src/styles/globals.css](/home/user/code/rust/starter/packages/starter-ui-kit/src/styles/globals.css)
- Existing theme-editor presets (phase 2 source data): [/home/user/code/rust/starter/packages/starter-ui-core/src/theme-editor/presets.ts](/home/user/code/rust/starter/packages/starter-ui-core/src/theme-editor/presets.ts)
- Existing dashboard widgets (phase 5 port source): [/home/user/code/rust/starter/packages/starter-ui-dashboard/](/home/user/code/rust/starter/packages/starter-ui-dashboard/)
- IR Kind union (phase 4 parity reference; 26 variants): [/home/user/code/rust/starter/packages/starter-ui-ir/src/index.ts](/home/user/code/rust/starter/packages/starter-ui-ir/src/index.ts)
- Smoke fixture (phase 4 acceptance): [/home/user/code/rust/starter/rubix/crates/rubix-flows/dashboards/disk-overview.json](/home/user/code/rust/starter/rubix/crates/rubix-flows/dashboards/disk-overview.json)
- Existing web SPA (regression target across every gate): [/home/user/code/rust/starter/rubix/frontend/](/home/user/code/rust/starter/rubix/frontend/)
- Workspace manifest (phase 2 + 3 + 4 + 5 registration): [/home/user/code/rust/starter/pnpm-workspace.yaml](/home/user/code/rust/starter/pnpm-workspace.yaml)
