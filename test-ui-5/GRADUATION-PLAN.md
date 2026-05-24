# test-ui-5 → packages graduation plan

Companion to [SCOPE.md](./SCOPE.md). SCOPE.md says patterns that earn
their keep here graduate into [packages/](../packages/). This doc is the
**how** for the first graduation pass.

**Status**: Stage 1 (i18n) **landed** as of 2026-05-24. Stage 2 Step A
(additive ui-core extensions) + Step C (test-ui-5 swap to packaged
store) **landed** the same day. Step B (port ConfigDrawer UI into
`starter-ui-kit`) **not started** yet — see [What's left](#whats-left)
below.

## Progress snapshot (2026-05-24)

### Done

- ✅ `react-intl` added to test-ui-5 (`package.json`).
- ✅ EN + ES message catalogs at [src/i18n/en.json](./src/i18n/en.json),
  [src/i18n/es.json](./src/i18n/es.json) — ~120 keys covering chrome,
  routes, a11y labels, plural rules for site counts.
- ✅ Locale zustand store with localStorage persistence
  ([src/i18n/index.ts](./src/i18n/index.ts)).
- ✅ `I18nProvider` wrapping `react-intl`'s `IntlProvider`
  ([src/i18n/provider.tsx](./src/i18n/provider.tsx)), mounted in
  [src/main.tsx](./src/main.tsx).
- ✅ Hardcoded strings extracted from:
  - [src/components/top-header.tsx](./src/components/top-header.tsx)
  - [src/components/action-dock.tsx](./src/components/action-dock.tsx)
  - [src/components/layout/nav-group.tsx](./src/components/layout/nav-group.tsx)
  - [src/components/layout/team-switcher.tsx](./src/components/layout/team-switcher.tsx)
  - [src/components/layout/data/sidebar-data.ts](./src/components/layout/data/sidebar-data.ts)
  - [src/lib/nav.ts](./src/lib/nav.ts) (now carries `labelKey`/`titleKey`)
  - [src/routes/settings.tsx](./src/routes/settings.tsx)
  - [src/routes/dashboard.tsx](./src/routes/dashboard.tsx)
  - [src/routes/index.tsx](./src/routes/index.tsx)
- ✅ EN/ES `LocaleMenu` in the ActionDock (visible right of the mode
  switcher).
- ✅ `tsc -b` clean. `vite build` clean (737 kB / 223 kB gzip).

### Deviation from original plan

- **Did not** wire `@nube/starter-ui-core/i18n` directly. ui-core's
  `IntlProvider` requires a `StarterClient` and an HTTP manifest
  endpoint (`/v1/i18n/manifest`) — wrong tradeoff for a sandbox with no
  server. Used `react-intl` directly with the same API surface
  (`useIntl().formatMessage`). Future swap to ui-core's provider is a
  one-line change in [src/main.tsx](./src/main.tsx) once
  test-ui-5 grows a `StarterClient` (or once ui-core grows a static-
  catalog mode).

### Stage 1 cleanup pass (also 2026-05-24)

- ✅ Widget sweep completed.
  [activity-feed.tsx](./src/components/dashboard/activity-feed.tsx)
  (titles, meta, "Living signal", "streaming", "now"),
  [performance-chart.tsx](./src/components/dashboard/performance-chart.tsx)
  ("Energy harvested"),
  [boot-intro.tsx](./src/components/boot-intro.tsx) ("Breathe · Drink
  · Grow"), and
  [layout-toggle.tsx](./src/components/layout-toggle.tsx) ("Header",
  "Sidebar") now all use `useIntl`.
- ✅ `metric-card.tsx`, `radial-progress.tsx`, `feature-tile.tsx`
  audited — pure prop-passthrough components, no hardcoded strings of
  their own.
- ✅ Catalogs grew to ~140 keys total. Both EN and ES updated.
- ✅ Dev server smoke test:
  `vite` boots clean on `localhost:5175`,
  `/`, `/src/main.tsx`, `/src/i18n/provider.tsx`, `/src/i18n/en.json`
  all serve 200, `IntlProvider` compiles. (No Chrome DevTools MCP in
  this session — visual verification still requires opening the
  browser manually.)
- ⏳ **Deferred to Stage 2**: the richer `ConfigDrawer`
  ([src/components/theme/config/](./src/components/theme/config/))
  carries ~80 hardcoded strings across `appearance`, `layout`,
  `branding`, `advanced` sections. These are intentionally **not**
  translated yet — the entire ConfigDrawer is slated to graduate into
  `starter-ui-kit` in Stage 2, where its strings will be re-keyed in
  one pass. Translating them now would be wasted work.

### Stage 2 progress (2026-05-24)

Resolution of the four open decisions, applied:

1. **Tailwind v4 lock-in**: accepted, but as an *opt-in* generator
   alongside the existing classic-CSS one. ui-core stays
   framework-agnostic by default.
2. **`LayoutPreferences`**: new sibling type (not part of
   `ShellConfig`). Kept in its own zustand store so the 38-token
   undo/redo doesn't pull layout knobs into its history.
3. **i18n catalog source**: settled in Stage 1 — static JSON.
4. **Palette-enum API on ui-core**: skipped. `LayoutPreferences`
   carries an opaque `palette: string | null`; the consumer owns the
   enumeration and matches on `[data-palette="..."]` in its own CSS.

**Step A — landed in ui-core** ([packages/starter-ui-core/src/theme-editor/](../packages/starter-ui-core/src/theme-editor/)):

- ✅ [layout-preferences.ts](../packages/starter-ui-core/src/theme-editor/layout-preferences.ts):
  `LayoutPreferences`, `defaultLayoutPreferences`, scales
  (`DENSITY_SCALE`, `FONT_SIZE_SCALE`), `resolveMode()`,
  `subscribePrefersDark()`, `subscribePrefersReducedMotion()`.
- ✅ [utils/apply-preferences.ts](../packages/starter-ui-core/src/theme-editor/utils/apply-preferences.ts):
  `applyThemePreferences()` writes `data-mode` / `data-palette` /
  `data-motion` / `data-density` / `data-font-size` attributes and the
  two CSS vars in one pass. Returns the resolved mode so callers can
  chain into `applyThemeToElement`.
- ✅ [utils/tailwind-css.ts](../packages/starter-ui-core/src/theme-editor/utils/tailwind-css.ts):
  `generateTailwindThemeCss()` opt-in TW v4 `@theme inline` generator.
- ✅ [layout-preferences-store.ts](../packages/starter-ui-core/src/theme-editor/layout-preferences-store.ts):
  zustand store via `createLayoutPreferencesStore()` (factory) +
  shared `useLayoutPreferences` singleton. Persists to localStorage
  by default; tests pass `storage: null` for isolation.
- ✅ All new surface re-exported from
  [theme-editor/index.ts](../packages/starter-ui-core/src/theme-editor/index.ts).
- ✅ [__tests__/layout-preferences.test.ts](../packages/starter-ui-core/src/theme-editor/__tests__/layout-preferences.test.ts):
  13 new test cases covering `resolveMode`, `applyThemePreferences`,
  `clearThemePreferences`, `generateTailwindThemeCss`, and the store
  (seed, setters, hydrate, custom initial).
- ✅ `vitest run` in ui-core: **100 / 100 passing**.
- ✅ Consumer typechecks clean: `starter-ui-kit`,
  `starter-ext-ui`, `starter-ext-sdk-ts`.

**Step C — landed in test-ui-5**:

- ✅ [package.json](./package.json) gained
  `@nube/starter-ui-core: workspace:*`.
- ✅ [src/stores/theme-store.ts](./src/stores/theme-store.ts)
  rewritten as a thin facade over ui-core's `useLayoutPreferences`
  for the shared concerns (mode / palette / density / motion /
  fontSize). A small ui-5-local zustand store carries the two enums
  that don't generalise into ui-core: `font` (Geist / Inter / Manrope
  / System) and `radius` (none / sm / md / lg). The public `useTheme()`
  shape is unchanged so route components don't need edits.
- ✅ [src/components/theme/theme-provider.tsx](./src/components/theme/theme-provider.tsx)
  now drives ui-core's `applyThemePreferences` for the shared knobs
  (one effect, one DOM write) and keeps the two ui-5-local effects for
  font + radius. Uses ui-core's `subscribePrefersDark` for the
  OS-level live-update path.
- ✅ `tsc -b` clean. `vite build` clean (793 kB / 241 kB gzip — +56kB
  vs. Stage 1 from pulling ui-core's theme-editor in, expected).
- ✅ Dev server boots clean on `localhost:5176`; theme-store and
  theme-provider modules serve 200 via Vite SSR-transform.

### What's left

**Step B — port ConfigDrawer UI into `starter-ui-kit`** (the original
plan's "biggest" step) is **not done** yet. The current `ConfigDrawer`
at [src/components/theme/config/](./src/components/theme/config/) is
still test-ui-5-local, with its strings still hardcoded English (the
~80-string set deferred from Stage 1). When Step B happens:

1. Port `ConfigDrawer` + section components (`appearance`, `layout`,
   `branding`, `advanced`) into
   `packages/starter-ui-kit/src/theme-editor/`.
2. Re-key all of its hardcoded strings through `@nube/starter-ui-core/i18n`.
3. Replace test-ui-5's local `ConfigDrawer` import with the packaged
   one.

Step B is a self-contained pass and a good unit of work for a future
session. The current Step A + C work is independently shippable: ui-5
now consumes ui-core's preference model, ui-core's API is additive
(no breaking changes), and every existing consumer still builds and
tests.


**Scope of this pass**: `i18n` and `theme-editor` only. Components,
layout, MF host scaffolding are explicitly deferred to a later pass.

---

## Audit findings

### i18n — clean one-way wiring

- test-ui-5 has zero i18n today (deferred per SCOPE.md).
- `@nube/starter-ui-core/i18n` is a complete react-intl stack:
  `IntlProvider`, `useTranslate`, manifest/catalog fetcher, BCP-47
  locale fallback, extension catalog registry, telemetry sink.
- Already consumed by `examples/flow-agent` and `examples/notes`.
- ~60–80 hardcoded user-facing strings in test-ui-5, concentrated in:
  - `src/routes/settings.tsx` (~17+)
  - `src/routes/dashboard.tsx` (~8)
  - `src/routes/index.tsx` (~10+)
  - `src/components/action-dock.tsx` (~7)
  - `src/components/layout/*` (~8 combined)
  - dashboard widget components (~8)

**Direction**: one-way. Wire ui-core's i18n into test-ui-5. Nothing to
merge back.

### theme-editor — genuine merge

Both implementations are real and have disjoint strong points.

| Capability                    | ui-core                          | test-ui-5                |
|-------------------------------|----------------------------------|--------------------------|
| Token model                   | 38 OKLCH tokens (full)           | 3-palette HSL enum       |
| Tailwind v4 `@theme` output   | no                               | yes (CSS-first)          |
| Mode                          | light / dark                     | light / dark / **system**|
| Density scale                 | —                                | compact/comfort/spacious |
| Font size scale               | —                                | sm/md/lg                 |
| Motion preference             | —                                | full/reduced + media qry |
| Undo/redo                     | 30-frame ring, collapse window   | —                        |
| Contrast checker (AA/AAA)     | yes                              | —                        |
| CSS import (parse `:root`)    | yes                              | —                        |
| Logo / favicon upload         | yes (tri-state)                  | placeholder only         |
| HTTP transport (server sync)  | yes                              | —                        |
| localStorage transport        | yes                              | yes (zustand persist)    |
| Preset gallery                | 10 full-theme swaps              | 3 palette-only swatches  |

**Existing consumers of `@nube/starter-ui-core/theme-editor` we must
not break**: `starter-ui-kit`, `examples/flow-agent`,
`starter-ext-ui`, `starter-ext-sdk-ts`, `examples/notes`.

**Direction**: extend ui-core additively with test-ui-5's modern
features. Do **not** delete ui-core's implementation. Do **not** create
a sibling package.

---

## Stage 1 — i18n into test-ui-5

Low-risk, mechanical. No new code in ui-core.

1. Add `@nube/starter-ui-core` to [package.json](./package.json) as a
   workspace dep.
2. Mount `IntlProvider` at [src/main.tsx](./src/main.tsx). No
   `PreferencesProvider` for the sandbox — pass `language` explicitly
   from a tiny zustand locale store.
3. Catalog source: **static EN/ES JSON** bundled in
   `src/i18n/{en,es}.json`. Bypass the manifest fetcher. The sandbox
   has no server.
4. Extract strings from the hotspot files above into keyed messages.
   Flat namespace: `dashboard.live`, `settings.layout.title`, etc.
5. Add an EN/ES toggle to the header — proves wiring end-to-end.

**Exit signal**: test-ui-5 boots in both locales; no hardcoded
user-facing English remains in `routes/` and `components/layout/`.

---

## Stage 2 — theme-editor merge

Strategy: **extend ui-core in place**, port test-ui-5 editor UI into
`starter-ui-kit`, then point test-ui-5 at the packaged versions.

### Step A — additive extensions to `@nube/starter-ui-core/theme-editor`

No breaking changes to existing public types.

1. New sibling type `LayoutPreferences` holding `density`, `motion`,
   `fontSize`. Kept separate from the 38-token `ThemeDocument` so the
   theme model stays clean.
2. Extend `mode` to `'light' | 'dark' | 'system'`. Add `resolveMode()`
   helper porting test-ui-5's `window.matchMedia` logic.
3. New `generateTailwindThemeCss()` alongside the existing
   `generateCssString()`. Existing consumers untouched; TW v4
   consumers opt in.
4. New `applyThemePreferences()` DOM helper — ports test-ui-5's
   `applyTheme` / `applyDensity` / `applyMotion` / `applyFontSize`
   verbatim, driven by attributes (`data-mode`, `data-palette`,
   `data-motion`) and CSS vars (`--density-scale`,
   `--font-size-scale`).

### Step B — port test-ui-5 editor UI into `@nube/starter-ui-kit`

5. Port `ConfigDrawer` (4-tab Appearance / Layout / Branding /
   Advanced) as a new component alongside the existing
   `ThemeEditorPage`. Consumers pick the form factor.
6. Port the section components (`PaletteConfig`, `DensityConfig`,
   `MotionConfig`, `FontSizeConfig`, etc.) under
   `starter-ui-kit/src/theme-editor/sections/`.

### Step C — point test-ui-5 at the packages

7. Replace [src/stores/theme-store.ts](./src/stores/theme-store.ts)
   with `useThemeEditorStore` from ui-core (extended).
8. Replace
   [src/components/theme/theme-provider.tsx](./src/components/theme/theme-provider.tsx)
   with the ui-core preferences-apply hook.
9. Replace `src/components/theme/config/*` with the ported
   `ConfigDrawer` from ui-kit.
10. Keep [src/styles/theme.css](./src/styles/theme.css) and
    [src/styles/tokens.css](./src/styles/tokens.css). The Tailwind v4
    CSS layer is consumer-side; ui-core just produces tokens that feed
    these CSS vars.

### Step D — consumer non-regression gate

11. Existing ui-core theme-editor tests stay green.
12. `examples/flow-agent` and `examples/notes` build and render
    unchanged.
13. `starter-ui-kit`, `starter-ext-ui`, `starter-ext-sdk-ts` build
    clean.

**Exit signal**: test-ui-5 renders identically (or better) using
packaged components; every existing consumer still builds and tests
pass.

---

## Out of scope this pass

- Graduating test-ui-5 components, layout, or MF host scaffolding into
  packages.
- Retiring `test-ui/` or `test-ui-3/`.
- Promoting test-ui-5 itself to a published package.
- Backfilling unit tests for the ported test-ui-5 sections (worth
  doing, but separate).

---

## Open decisions

Pending the user's call before code lands:

1. **Tailwind v4 lock-in**: adding `generateTailwindThemeCss()` is
   opt-in, but density / motion / font-size scaling assumes TW v4 in
   the consumer. Acceptable?
2. **`LayoutPreferences` as a new type vs. extending `ShellConfig`**:
   leaning new type — `ShellConfig` is currently about branding and
   feature flags, layout prefs don't belong there.
3. **i18n catalog source**: static JSON in test-ui-5 (Option A) vs.
   wiring the server fetcher (Option B). Leaning A.
4. **Palette-enum API on ui-core**: probably skip — test-ui-5 can call
   `setStyles(palettePresets[id])` against the existing store. Less
   API surface, same outcome.

---

## Order of operations

1. Resolve [Open decisions](#open-decisions).
2. Land [Stage 1](#stage-1--i18n-into-test-ui-5). Smaller, mechanical,
   unblocks itself.
3. Land [Stage 2](#stage-2--theme-editor-merge) Step A (additive
   ui-core extensions) — non-breaking, ships independently.
4. Step B (port editor UI into ui-kit).
5. Step C (swap test-ui-5 to packaged versions).
6. Step D (consumer non-regression sweep).
