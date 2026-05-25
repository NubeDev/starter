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
starter-ui-sdui-native   ← starter-ui-sdui-react (registry), starter-ui-ir
        ↑
starter-ui-dashboard-native ← starter-ui-kit-native, react-native-svg
```

`starter-theme-tokens` is consumed by **both** `starter-ui-kit`
(web) and `starter-ui-kit-native` (mobile) so colour, density, and
type scale are identical by construction. The web kit's CSS is
generated from the same object; that refactor is part of the
package-1 PR.

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
package and emits the same CSS vars. Zero behaviour change on web.

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
primitives. **MUST NOT:** import `starter-ui-kit` (no web deps),
do network I/O, or own application state.

**Choice of foundation:** start on RN core + `react-native-svg` +
`moti`. If the per-component port cost is high, evaluate Tamagui
or gluestack-ui as a *implementation detail* — the API surface
this package exposes does not change.

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
[`packages/starter-ui-sdui-react/src/renderer/index.ts`](../../../../packages/starter-ui-sdui-react/src/renderer/index.ts).
**MUST NOT:** import `starter-ui-kit` or any web-only package.

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
