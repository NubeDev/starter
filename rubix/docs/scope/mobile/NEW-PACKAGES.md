# Mobile — new workspace packages

Four new packages, each with one responsibility, mirroring the
verb-per-file discipline of [FILE-LAYOUT.md](../../../FILE-LAYOUT.md).
They live under `packages/` (not `rubix/packages/`) because they
are starter-level chassis pieces: the next starter-based mobile
app reuses them unchanged, the same way `rubix/frontend` reuses
`starter-ui-kit`.

## Package map

```
packages/
  starter-theme-tokens/         ← token values, JS object
  starter-ui-kit-native/        ← RN primitives, mirrors ui-kit API
  starter-ui-sdui-native/       ← RN renderers, registers into sdui-react
  starter-ui-dashboard-native/  ← RN ports of MetricCard et al.
```

Dependency arrow:

```
starter-theme-tokens     ← no deps
        ↑
starter-ui-kit-native    ← react-native, react-native-svg, moti
        ↑
starter-ui-sdui-native   ← starter-ui-kit-native, starter-ui-ir,
                             starter-ui-sdui-react/headless (registry)
        ↑
starter-ui-dashboard-native ← starter-ui-kit-native, react-native-svg
```

`starter-theme-tokens` is consumed by **both** `starter-ui-kit`
(web) and `starter-ui-kit-native` (mobile) so colour, density, and
type scale are identical by construction. The web kit's CSS is
generated from the same object; that refactor is part of the
package-1 PR.

## Precondition — sdui-react package split

This whole plan blocks on `@nube/starter-ui-sdui-react` exposing a
`./headless` subpath that contains `SduiPage`, `SduiProvider`, the
hooks, the transport, and the renderer registry — **without**
re-exporting `./renderer/*` (which today's root barrel does, and
which would pull `@nube/starter-ui-kit` into the mobile bundle).
The registry must move into `/headless` so web and mobile share
one module instance — a second copy would silently de-register.

Three concrete refactors are required, in this order:

1. **Move the registry** (`renderer/registry.ts`) under
   `headless/`. Web renderers update their import path; the
   registry itself doesn't change.
2. **Move `sdui-page.tsx` to import the registry directly.**
   Today it imports `Render, listRenderers from "./renderer/index.js"`
   — the barrel, which triggers every web renderer's
   `registerRenderer(...)` side-effect. The refactor switches it
   to `./headless/registry.js` (or whatever the new location is)
   so that importing `/headless` from mobile pulls **only** the
   registry + page logic, not any renderers.
3. **Add the `./headless` export entry** to
   `packages/starter-ui-sdui-react/package.json` and update the
   root barrel to re-export from headless instead of duplicating
   the API surface.

Proposed shape:

```
@nube/starter-ui-sdui-react/headless   ← SduiPage, SduiProvider, hooks,
                                          transport, registerRenderer/
                                          lookupRenderer/listRenderers
@nube/starter-ui-sdui-react             ← today's root; web renderers,
                                          which depend on /headless
```

Web consumers continue to import the root; mobile imports only
`/headless`. Until this lands, [Block 3](./THIN-SLICE.md#block-3--nubestarter-ui-sdui-native-first-five-kinds)
cannot begin.

The registry refactor + `sdui-page.tsx` decoupling is itself a
public-API change to a starter package and is recorded as a
consequence in
[ADR 0004](../../adr/0004-react-native-mobile-app.md#consequences).

---

## starter-theme-tokens

**Owns:** the source-of-truth values for palettes, density,
radii, font sizes, motion scales, and semantic role → token
mappings. A single JS object.

**Today's source data:**

- [`packages/starter-ui-kit/src/styles/globals.css`](../../../../packages/starter-ui-kit/src/styles/globals.css) — colour vars + density tokens.
- [`packages/starter-ui-core/src/theme-editor/presets.ts`](../../../../packages/starter-ui-core/src/theme-editor/presets.ts) — named preset palettes.

The values move to `packages/starter-theme-tokens/src/`, one file
per concept:

```
src/
  index.ts            ← barrel, re-exports only
  palette.ts          ← named palettes, HSL triplets
  density.ts          ← spacing scale, control sizes
  radius.ts           ← border radius scale
  type.ts             ← font sizes, weights, line heights
  motion.ts           ← duration + easing scales
  role.ts             ← semantic role → token mapping
```

**Web migration:** `starter-ui-kit`'s `globals.css` becomes a
build-time generator (`scripts/generate-css.ts`) that reads this
package and emits the same CSS vars. Zero behaviour change on web
is the goal; **validation** is by visual diff. Visual-snapshot CI
for `starter-ui-kit` does not exist today; Block 1 lands either
(a) a minimal snapshot harness (Playwright + per-primitive page),
or (b) a documented manual-review checklist signed off in the PR.
Pick (a) if Block 2 will use it too.

**MUST:** be pure data. **MUST NOT:** depend on React, RN, DOM, or
any styling runtime.

---

## starter-ui-kit-native

**Owns:** React Native primitives whose **prop API mirrors
`@nube/starter-ui-kit`** one-to-one. Mirror means: if
`<Button variant="outline" size="sm" onClick={…}>` works on web,
`<Button variant="outline" size="sm" onPress={…}>` works on mobile
with the same visual result.

**First-cut surface** (only what dashboards need; everything else
is YAGNI until a renderer asks for it):

```
src/
  button.tsx
  card.tsx
  input.tsx
  tabs.tsx
  badge.tsx
  switch.tsx
  slider.tsx
  select.tsx
  sheet.tsx       ← native bottom sheet, replaces Radix Sheet
  dialog.tsx
  spinner.tsx
  skeleton.tsx
  tooltip.tsx
```

One verb per file. No `primitives.tsx`, no `index.tsx` with bodies.

**Theming:** every component reads tokens via a `useTheme()` hook
backed by `starter-theme-tokens` + the layout-preferences store
from `starter-ui-core/theme-editor`. No `className`, no
`StyleSheet.create()` calls outside the component file that uses
them.

**MUST:** match the `starter-ui-kit` component API for the listed
primitives. Every primitive ships with appropriate
`accessibilityRole` and `accessibilityLabel` / `accessibilityHint`
props wired through to the RN base element — this is a **kit
acceptance criterion**, not a polish item. A reviewer is
entitled to block a primitive PR that ships a `Pressable` without
`accessibilityRole="button"` or a `TextInput` without an
`accessibilityLabel` resolution path.
**MUST NOT:** import `starter-ui-kit` (no web deps),
do network I/O, or own application state.

**Choice of foundation:** RN core + `react-native-svg` + `moti`.
A later swap to Tamagui or gluestack-ui is **not** an
implementation detail — it changes the styling runtime model
and the snapshot baseline — so it would require its own ADR. The
scope plan commits to the RN-core path; do not adopt a styling
framework as a Block-2 deviation.

---

## starter-ui-sdui-native

**Owns:** one renderer per IR `Kind`, registered into the same
`registerRenderer` registry that
[`@nube/starter-ui-sdui-react`](../../../../packages/starter-ui-sdui-react/)
ships. One file per kind, matching the web layout exactly:

```
src/
  index.ts              ← register-all entry point (barrel)
  render-page.tsx
  render-grid.tsx
  render-row.tsx
  render-col.tsx
  render-kpi.tsx
  render-chart.tsx
  render-divider.tsx
  render-tabs.tsx
  render-table.tsx
  render-form.tsx
  render-select.tsx
  render-slider.tsx
  render-toggle.tsx
  render-date-range.tsx
  render-repeat.tsx
  render-custom.tsx
```

Each `render-<kind>.tsx` is ≤150 lines and uses **only**
`starter-ui-kit-native` + `starter-ui-ir` types. No direct RN
primitives in renderer files — that keeps the styling consistent
and the renderer testable against a swap-in mock kit.

Registration is import-once: the mobile app imports
`@nube/starter-ui-sdui-native` for its side effects (the barrel
calls `registerRenderer(...)` for every kind), then mounts the
existing `<SduiPage>` from `starter-ui-sdui-react`.

**MUST:** cover the same kinds in the same priority order as
[`packages/starter-ui-sdui-react/src/renderer/index.ts`](../../../../packages/starter-ui-sdui-react/src/renderer/index.ts)
(16 kinds today). **MUST NOT:** import `starter-ui-kit` or any
web-only package.

**Parity vs the IR `Kind` union (26 variants).** The IR declares
26 kinds in
[`packages/starter-ui-ir/src/index.ts`](../../../../packages/starter-ui-ir/src/index.ts);
the web renderer registers 16. The 10 unimplemented-on-web kinds
are:
`stack, card, text, heading, badge, kpi_grid, button, link, field, sparkline`.
Mobile **inherits the same 16 today** and **defers the same 10**
until web ships them — a parity backfill, web first. The
rationale is simple: a phone surface should not silently render
IR shapes that the source-of-truth web renderer rejects. If
mobile ever needs one of the 10 before web does, that's a
separate decision and gets called out in the PR.

---

## starter-ui-dashboard-native

**Owns:** RN ports of the four dashboard widgets in
[`packages/starter-ui-dashboard/`](../../../../packages/starter-ui-dashboard/),
with **identical prop APIs** so a feature consumed on web can ship
on mobile by changing only the import.

One file per widget:

```
src/
  metric-card.tsx
  radial-progress.tsx
  activity-feed.tsx
  performance-chart.tsx
```

**Mapping rules:**

- `<div>` / Tailwind classes → `<View>` + `StyleSheet`.
- Inline `<svg>` → `react-native-svg`.
- `motion/react` animations → `moti` (which sits on
  `react-native-reanimated`).
- Colour and spacing come from `starter-theme-tokens` via the
  same `useTheme()` hook the kit uses.

**MUST:** match the prop API of the corresponding `starter-ui-dashboard`
component exactly. **MUST NOT:** add mobile-only props; if mobile
needs a new prop, add it to the web component first.
