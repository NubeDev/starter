# Mobile — UI foundation options (Tamagui vs. keep current vs. gluestack)

> Written 2026-05-25 after the kit-native package landed on
> `@nube/starter-ui-kit-native` (13 primitives + tests) and the
> Expo SDK 54 app shell shipped under `rubix/mobile/`. The
> question on the table: **should we replace the hand-rolled
> `starter-ui-kit-native` foundation with [Tamagui](https://tamagui.dev/)?**

**TL;DR recommendation:** **No, not now.** Keep
`starter-ui-kit-native` as-is (RN core + `react-native-svg` +
`moti`). The cost of swapping is high, the benefit is mostly
"things we already have", and the one real Tamagui upside
(unified web + native primitives) actively conflicts with the
existing web kit's Tailwind/shadcn surface. The defensible
alternative if a11y / perf / animation quality becomes a real
problem is **gluestack-ui v2** (copy-paste primitives, same
"own the code" philosophy as shadcn). Full reasoning and the
opt-in scope for Tamagui are below.

## Current state (verified 2026-05-25)

- `packages/starter-ui-kit-native/` exists, 13 primitives shipped
  (`button`, `card`, `input`, `tabs`, `badge`, `switch`, `slider`,
  `select`, `sheet`, `dialog`, `spinner`, `skeleton`, `tooltip`),
  each with a `*.test.tsx` and a11y assertion. Foundation: RN
  core (`Pressable`, `Text`, `View`, `StyleSheet`) + `moti` +
  `react-native-svg`. Reads tokens via `useTheme()` →
  `@nube/starter-theme-tokens`.
- `packages/starter-ui-sdui-native/` registers 16 renderers
  against the shared `starter-ui-sdui-react/headless` registry.
  Each renderer imports **only** `starter-ui-kit-native` + IR
  types — no direct RN primitives.
- `packages/starter-ui-dashboard-native/` ports the 4 dashboard
  widgets.
- `rubix/mobile/` is on **Expo SDK 54** (`expo: ~54.0.0`,
  `react: 19.1.0`, `react-native: 0.81.5`, `react-native-reanimated:
  ~4.1.7`, `react-native-worklets: ^0.5.1`). App shell, login,
  connections ledger, providers stack — all landed.
- Web kit (`@nube/starter-ui-kit`) is Tailwind + Radix. The
  `starter-theme-tokens` extraction is the **single source of
  truth**: web `globals.css` is generated from tokens and is
  byte-identical to the previous hand-written file. Mobile
  reads the same tokens.

## Option A — Keep current (recommended)

**What it is:** carry on with `starter-ui-kit-native`. Spend
the next chunk of effort on **hardening** (a11y audit,
performance with `React.memo` / `useMemo` discipline, animation
polish via moti), and on building Block 5 (`<SduiPage>` + the
`/dashboards/[pageId]` body) which is the actual user-visible
slice.

**Pros**
- Zero rewrite. Every component shipped today keeps shipping.
- Single token system (`starter-theme-tokens`) stays
  authoritative across web + native.
- Web kit (Tailwind/shadcn/Radix) is undisturbed — the
  byte-identical CSS invariant from the chassis job stays
  intact.
- Renderer files in `starter-ui-sdui-native` stay ≤150 lines and
  import only kit-native — that contract is the lint surface the
  whole IR seam was designed around.
- No new compile-time toolchain (Tamagui's babel/metro plugin,
  config file, `tamagui.config.ts`). Boot time and CI complexity
  unchanged.
- Honours the established "own the code" philosophy (same as
  the web kit shipping copy-paste shadcn primitives).

**Cons**
- We maintain 13 primitives ourselves. Each one's a11y, RTL,
  and animation polish is on us.
- No cross-platform component story — a feature shipped on web
  with `<Button>` from `starter-ui-kit` and on mobile with
  `<Button>` from `starter-ui-kit-native` is two
  implementations with matched APIs, not one.
- No compile-time style extraction; styles are computed at
  render time. Not a measured problem today, but a theoretical
  perf ceiling.

**When to revisit:** if a primitive's a11y or perf becomes a
real blocker that copy-paste maintenance can't keep up with, OR
if a second mobile app inside the workspace wants to share
primitives.

## Option B — Adopt Tamagui (scope below)

**What it is:** delete the body of `starter-ui-kit-native` and
re-export Tamagui primitives wrapped to match the existing prop
API. Keep the package name and the public surface so consumers
(`starter-ui-sdui-native`, `starter-ui-dashboard-native`,
`rubix/mobile`) don't change.

**Pros**
- Battle-tested primitives, larger surface area than we'd build
  (over 100 components including `XStack`, `YStack`, `ListItem`,
  `Sheet`, `Popover`, `Toast`, `Switch`, `Avatar`, etc.).
- Compile-time style extraction → atomic CSS on web, flatter
  view tree on native. Measurable perf wins on long lists.
- Strong built-in animation primitives (`animation="bouncy"`)
  that wrap reanimated and avoid hand-wiring moti per primitive.
- A11y contracts (role, label, focus management) are
  Tamagui's responsibility, not ours.
- Theming model (tokens, themes, sub-themes, media tokens) is
  Tamagui-native and well-trodden.

**Cons (the honest ones)**
- **Web kit divergence.** Tamagui's web output is its own CSS
  layer. The web SPA (`rubix/frontend`) ships Tailwind +
  shadcn + Radix and is **not** going to migrate. So Tamagui
  on mobile means we now have **two** UI systems in the
  monorepo with two token languages — the opposite of what
  `starter-theme-tokens` was extracted to fix. The byte-
  identical `globals.css` invariant from the chassis job
  becomes irrelevant on mobile and a wedge between the two
  surfaces.
- **Token reconciliation.** Tamagui's `createTokens()` /
  `createThemes()` config format does NOT map 1:1 onto
  `starter-theme-tokens` shape (HSL triplets, density steps,
  role colours). We'd ship a generator from
  `starter-theme-tokens` → `tamagui.config.ts` and maintain it
  — non-trivial, and another place visual drift can creep in.
- **Rewrite cost.** 13 primitives, each with tests, each
  re-implemented as a Tamagui wrapper preserving the existing
  prop API exactly. `starter-ui-sdui-native`'s 16 renderers
  don't change (they import by name), but every primitive's
  test gets rewritten and every visual assumption re-validated.
  Realistic effort: a week of focused work, plus a visual
  re-review pass.
- **Compile-time toolchain.** Tamagui requires `@tamagui/babel-plugin`
  in the babel config and `@tamagui/metro-plugin` (or the
  optimizer) for production builds. Expo SDK 54 / RN 0.81 is
  **brand new** (Nov 2025) — Tamagui's compatibility matrix
  lags Expo by 1–2 minor versions historically. Verify
  before committing.
- **Lock-in.** Once renderers and screens are written against
  `<XStack>`, `<YStack>`, `styled()`, `<Theme name="dark">`,
  un-picking that is the same scale of work as picking it up.
- **Animation re-wire.** Moti is already in the tree
  (`moti: ^0.30.0`) and used inside kit-native. Tamagui ships
  its own animation API. Either both stay (extra dep weight)
  or moti gets removed (more rewrite).
- **No actual user-visible win.** Block 5 (`<SduiPage>` body)
  blocks on having any primitive set, not on having the *best*
  primitive set. Switching now delays Block 5.

**Acceptance bar IF we did it:** see [§Scope for Option B](#scope-for-option-b-tamagui-adoption-if-chosen)
below.

## Option C — Adopt gluestack-ui v2

**What it is:** [gluestack-ui v2](https://gluestack.io/ui/docs/home/overview/introduction)
ships as copy-paste primitives — same shape as shadcn for web.
You install the CLI, run `npx gluestack-ui add button`, and the
component lands in your repo as TS source you own. Built on
top of `react-native-aria` (RN port of `react-aria`) and Tailwind
via NativeWind.

**Pros**
- "Own the code" model matches the web kit's shadcn pattern
  exactly. Same mental model on both sides of the seam.
- `react-native-aria` foundation means a11y is real, not
  aspirational.
- Tailwind / NativeWind for styling means tokens map to
  utility classes that *look like* the web Tailwind config
  (still not identical, but closer than Tamagui).
- No compile-time toolchain beyond what NativeWind already
  needs.
- Per-primitive opt-in: we could swap `<Dialog>` and `<Sheet>`
  to gluestack while keeping the other 11 as-is, then
  evaluate.

**Cons**
- NativeWind adds a styling layer (config + babel plugin).
  Token reconciliation problem still exists, just smaller.
- Still a rewrite of the primitives we'd swap.
- Less compile-time perf magic than Tamagui.
- Migration noise in the file tree (the copy-paste files are
  ~200–400 lines each).

**When to consider:** if a specific primitive (sheet, dialog,
tooltip, popover — the focus-management-heavy ones) starts
costing us real bug-hours. Swap that one to gluestack, leave
the rest alone.

## Recommendation, plainly

1. **Now:** Option A. Ship Block 5. The chassis works; the
   user-visible slice doesn't exist yet.
2. **If a primitive bleeds:** Option C, scoped to the one
   primitive. Per-primitive opt-in preserves the current
   token system and the renderer contracts.
3. **Tamagui:** only if a future mobile app (a second consumer
   of `starter-ui-kit-native`) demonstrably needs compile-time
   style extraction for a list-heavy / chart-heavy surface
   that profiling shows is bottlenecked on RN's render path.
   That is not where we are. If it ever becomes where we are,
   the scope below is the starting point.

## Scope for Option B — Tamagui adoption (if chosen)

> Read this as the **draft codeless job** if the decision lands
> on Tamagui. Mirrors the rigour of `.codeless/jobs/mobile-chassis/`.

### Goal

Replace the body of `@nube/starter-ui-kit-native` with Tamagui-
backed implementations while **preserving the public API
verbatim** (every export name, every prop name, every prop
type). Downstream consumers (`starter-ui-sdui-native`,
`starter-ui-dashboard-native`, `rubix/mobile/src/**`) must not
need any source change beyond a workspace re-install.

After this scope merges:

1. `@nube/starter-ui-kit-native` re-exports Tamagui-backed
   primitives with unchanged prop API and unchanged test
   coverage shape (the assertions move, the assertions don't
   weaken).
2. `tamagui.config.ts` is generated from
   `@nube/starter-theme-tokens`. The generator is a script
   under `packages/starter-ui-kit-native/scripts/` and a CI
   check fails if `tamagui.config.ts` drifts from the
   regenerated output.
3. `rubix/mobile/babel.config.js` and `metro.config.js`
   include the Tamagui plugins, with documented diff against
   the previous configs.
4. `rubix/mobile` boots on iOS + Android, login works,
   connections list renders, dashboards stub resolves —
   identical to today.
5. A11y test surface is at least as strong as today (every
   primitive's `accessibilityRole` and `accessibilityLabel`
   assertion is preserved or replaced with an equivalent
   Tamagui assertion).

### In scope (six phases)

**Phase 0 — Compatibility probe (halt-gate).**
Verify Tamagui supports Expo SDK 54 / RN 0.81 / React 19.1
*today*. If it doesn't, halt. Concrete checks:

- `tamagui` + `@tamagui/config` install cleanly in
  `rubix/mobile` against the current `package.json`.
- A throwaway `<XStack>` + `<Button>` renders in the Expo dev
  client on both iOS and Android simulators.
- `@tamagui/babel-plugin` + `@tamagui/metro-plugin` don't
  break the existing `expo-router` v6 boot.
- The animation runtime works with `react-native-reanimated
  ~4.1.7` and `react-native-worklets ^0.5.1`.

Output: a `PHASE-0-PROBE.md` under
`.codeless/jobs/mobile-tamagui/` listing each check + pass/fail
+ versions tested. **Do not start phase 1 if any check fails;
escalate.**

**Phase 1 — Token bridge.**
Write `packages/starter-ui-kit-native/scripts/generate-tamagui-config.ts`.
Reads `@nube/starter-theme-tokens` (palette, density, radius,
type, motion, role) and emits a `tamagui.config.ts` that
exposes the same role colours, the same spacing scale, the
same radius tokens, the same type ramp. Commit a regression
fixture (`tamagui.config.expected.ts`) and a vitest that
fails on drift. The generated `tamagui.config.ts` is checked
in (not generated at build time — the CI check is a guard,
not a build step).

**Phase 2 — Wrap-and-replace, primitive by primitive.**
For each of the 13 primitives, in dependency order
(`badge → card → spinner → skeleton → button → input → switch
→ slider → select → tabs → tooltip → sheet → dialog`):

1. Rewrite the primitive's body to delegate to Tamagui's
   equivalent (e.g. `<Button>` → wraps Tamagui's `<Button>` +
   `<ButtonText>` mapping `variant`/`size` props to Tamagui's
   `themed`/`size` props).
2. Keep the export name and prop type unchanged. The diff is
   "what the file does inside", not "what the file looks like
   from outside".
3. Update the primitive's `*.test.tsx` to assert the SAME
   contracts (role, label, callback firing) — if the
   assertion can't be expressed against Tamagui's render
   tree, escalate; don't weaken the assertion.
4. One commit per primitive. Reviewer can revert any one
   primitive independently.

Each primitive's commit body MUST include:
- `pnpm --filter @nube/starter-ui-kit-native test` output
  (green).
- A before/after screenshot of the example harness (light +
  dark, default palette).

**Phase 3 — Renderer + dashboard regression.**
Since renderers and dashboards import primitives by name, no
source change there. Run the full suite:

- `pnpm --filter @nube/starter-ui-sdui-native test` — green.
- `pnpm --filter @nube/starter-ui-dashboard-native test` —
  green.
- The `disk-overview.json` smoke fixture renders in a unit
  test through the new Tamagui-backed primitives.

If any renderer was reaching into a primitive's private prop
(e.g. style escape hatch), capture it in
`.codeless/jobs/mobile-tamagui/RENDERER-DELTAS.md` with the
fix.

**Phase 4 — Mobile app integration.**
Wire `<TamaguiProvider config={config}>` into
`rubix/mobile/src/providers.tsx` (above or below the existing
`ThemeProvider` — decide based on the token-source-of-truth
rule: TamaguiProvider must consume the same tokens our
`ThemeProvider` already publishes). Update
`rubix/mobile/babel.config.js` and `metro.config.js` per
Tamagui's setup guide; commit the diff and document why each
plugin entry is there. Boot iOS sim + Android sim, walk:

- Login → connection add → bearer install.
- Connections list → switch active → cache namespacing intact.
- Dashboards stub → opens, records last page.
- Settings → logout.

Each walkthrough captured as a transcript or recording.

**Phase 5 — Drop or keep moti.**
Decision point with a written justification under
`.codeless/jobs/mobile-tamagui/MOTI-DISPOSITION.md`:

- Option (i): Tamagui's animation API covers every motion we
  use → remove `moti` from kit-native and rubix/mobile.
- Option (ii): Some motion (e.g. card press feedback) is
  better expressed in moti and the dep weight is acceptable
  → keep moti, document the boundary (which animations use
  which system, by name).

Reviewer signs off on the chosen option.

### REVIEW gates

- **Gate 1 (after phase 0):** compatibility probe is green
  OR the job halts.
- **Gate 2 (after phase 2):** all 13 primitives ported,
  example harness side-by-side (Tamagui vs. master)
  shows no visual regression on three breakpoints (320 /
  390 / 428) and light + dark. Reviewer signs the checklist
  per-primitive.
- **Gate 3 (after phase 4):** end-to-end app walkthrough
  green on iOS + Android sims; renderer + dashboard tests
  green; the token-drift CI guard is green.

### Out of scope

- **Migrating the web kit to Tamagui.** Web stays
  Tailwind + shadcn + Radix. If a future ADR decides to
  unify, that is its own job.
- **Swapping moti out before the phase-5 decision.** Moti
  stays in the tree through phases 0–4.
- **Re-theming.** Tokens, palettes, density, radius — all
  unchanged. The visual output should match master.
- **Refactoring renderer files.** Renderers import by name;
  they should not need to change. If one does, capture the
  delta and decide whether the primitive's wrapper should
  hide it.
- **Tamagui's optimizing compiler (`@tamagui/static`)** —
  evaluate in a follow-up perf job once integration is
  proven.
- **Web preview via Expo Web** — explicit non-goal per
  [NON-GOALS.md](./NON-GOALS.md).

### Constraints

- Public API of `@nube/starter-ui-kit-native` is FROZEN for
  this scope. No prop additions, no prop renames, no new
  exports. The only commit message verb is "swap", not "add".
- `@nube/starter-theme-tokens` is the only source of truth
  for colours, spacing, radius, type, motion, role. Tamagui
  tokens are *derived*. A handwritten edit to
  `tamagui.config.ts` is a CI failure.
- Every primitive's a11y assertion must be at least as
  strong as today.
- No `--no-verify`, no `// @ts-ignore` on workspace
  boundaries.
- File-size budget per [FILE-LAYOUT.md](../../../../FILE-LAYOUT.md):
  ≤400 lines per file, ≤50 lines per function.

### Cap suggestion

`cost_cap_cents: 40000`, `wall_clock_cap_ms: 21600000` (6h).
Phase 0 is the cheapest and the highest-information phase;
if it fails, the job halts with most of the cap unspent.

### Open questions to resolve before submitting

- **Q1.** Does Tamagui currently support Expo SDK 54 +
  React 19.1 + RN 0.81 in production? (Phase 0 answers this.
  If "no", don't submit the job.)
- **Q2.** Do we want to land this on `codeless/mobile-tamagui`
  and merge as one PR, or land it as 13 small PRs from a
  long-running branch? (Default: one branch, 13+ commits, one
  PR.)
- **Q3.** Is the user prepared to pause Block 5 (`<SduiPage>`
  body) while this lands? (If "no", that's the strongest
  argument for Option A or Option C.)

## References

- Current kit-native: [packages/starter-ui-kit-native/](../../../../packages/starter-ui-kit-native/)
- Current button primitive (example shape): [packages/starter-ui-kit-native/src/button.tsx](../../../../packages/starter-ui-kit-native/src/button.tsx)
- Token source of truth: [packages/starter-theme-tokens/](../../../../packages/starter-theme-tokens/)
- Mobile app shell: [rubix/mobile/](../../../mobile/)
- Mobile package manifest (Expo SDK 54 lock): [rubix/mobile/package.json](../../../mobile/package.json)
- Web kit (Tailwind/shadcn — explicitly not migrating): [packages/starter-ui-kit/](../../../../packages/starter-ui-kit/)
- Mobile non-goals (includes "no Expo Web"): [NON-GOALS.md](./NON-GOALS.md)
- New-packages spec (kit-native foundation rationale): [NEW-PACKAGES.md](./NEW-PACKAGES.md)
- Reuse matrix: [REUSE.md](./REUSE.md)
- Mobile chassis job (the work that built today's foundation): [.codeless/jobs/mobile-chassis/](../../../../.codeless/jobs/mobile-chassis/)
- Tamagui homepage: https://tamagui.dev/
- Gluestack-ui v2: https://gluestack.io/ui/docs/home/overview/introduction
