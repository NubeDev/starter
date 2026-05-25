# Mobile — reuse matrix

Every workspace package that `rubix/mobile` depends on, with the
exact reason it is portable. If a package is not in the **reused**
section, mobile MUST NOT import it.

## Reused unchanged — `workspace:*` dependency

| Package | Why portable |
|---|---|
| [`@nube/starter-client-ts`](../../../../packages/starter-client-ts/) | `globalThis.fetch` + zod; no React, no DOM. |
| [`@nube/starter-client-react`](../../../../packages/starter-client-react/) | React + react-query only. `react-dom` peer dep is convention; nothing in `src/hooks/*` imports it. SSE hook needs `react-native-sse` polyfill (see [APP-SHELL.md](./APP-SHELL.md)). |
| [`@nube/starter-ui-ir`](../../../../packages/starter-ui-ir/) | IR type mirror of Rust SPI. Zero deps. |
| [`@nube/rubix-client-ts`](../../../../../rubix/packages/rubix-client-ts/) | Wraps `starter-client-ts`. |
| [`@nube/rubix-client-react`](../../../../../rubix/packages/rubix-client-react/) | React-query hooks. Same React-only argument. |

## Reused — named exports from a few subpaths

`@nube/starter-ui-core` ships subpath exports; mobile imports a
narrow set of **named exports** from each (not whole subpaths —
some contain DOM-bound siblings). Path-level forbids are listed
in [APP-SHELL §Import lint](./APP-SHELL.md#import-lint).
**The named-export discipline below is not lint-enforceable today**
(`no-restricted-imports` is path-level); see the
[named-export caveat in APP-SHELL](./APP-SHELL.md#import-lint)
for the picked enforcement story (code review with CODEOWNERS
or a custom AST rule, decided at Block 4).

**Allowed:**

- `@nube/starter-ui-core/auth` — `AuthProvider`, `useAuth`, the
  pluggable `AuthStrategy` type, and **`tokenStrategy` only**.
  Mobile picks the token strategy. `externalStrategy` calls
  `window.location.assign` and is web-only;
  `sessionStrategy` is cookie + CSRF and is web-only.
- `@nube/starter-ui-core/query` — `starterQueryKey`. Mobile uses
  this to **namespace cache keys by active connection id** so
  switching servers doesn't bleed cache (see
  [APP-SHELL.md](./APP-SHELL.md#provider-stack)).
- `@nube/starter-ui-core/i18n` — locale store, manifest loader,
  fetcher, fallback. `react-intl` runs on RN; pin minimum Hermes
  per [APP-SHELL.md](./APP-SHELL.md#required-rn-runtime-deps).
- `@nube/starter-ui-core/preferences` — the **types,
  formatters, units, and store** only. The `PreferencesProvider`
  React component writes `document.documentElement.lang` / `.dir`
  and is **forbidden** (see below). Mobile re-implements the
  provider against RN's `I18nManager`.
- `@nube/starter-ui-core/theme-editor` — `types`, `defaults`,
  `presets`, the editor `store`, `layout-preferences` **store and
  types**, and the pure helpers `utils/color-converter` and
  `utils/contrast-checker`.
  The `layout-preferences` reader is **already guarded** —
  verified in
  [`packages/starter-ui-core/src/theme-editor/layout-preferences.ts`](../../../../packages/starter-ui-core/src/theme-editor/layout-preferences.ts):
  every `matchMedia` site is gated by
  `typeof window === "undefined" || typeof window.matchMedia !== "function"`,
  which fires under Hermes (`window` is polyfilled but
  `matchMedia` is not), returning the documented `"light"` /
  no-listener fallback. Mobile augments this by initialising the
  store from `Appearance.getColorScheme()` and updating on
  `Appearance.addChangeListener` — the guarded reader never
  fires on RN, so this is additive, not a replacement.
  These helpers are not currently exposed as sub-subpath exports;
  they are imported as named re-exports from `./theme-editor`.
  See the [named-export caveat](./APP-SHELL.md#import-lint).

**Forbidden:**

- `@nube/starter-ui-core/layout` — `cookies.ts` reads
  `document.cookie`; `use-mobile.ts` uses `matchMedia`. Mobile
  replaces both: bearer-token auth via `expo-secure-store`,
  breakpoints via `useWindowDimensions()`.
- `PreferencesProvider` (the React component in `…/preferences`) —
  writes `document.documentElement.lang` / `.dir`. Mobile uses
  RN's `I18nManager.forceRTL` and `Localization` instead.
- The `localStorageThemeTransport` from `theme-editor/transport.ts` —
  uses `window.localStorage`. Mobile supplies an AsyncStorage-backed
  `ThemeTransport` instead.
- The `apply-*` helpers in `theme-editor/utils/` (`apply-theme.ts`,
  `apply-preferences.ts`, `generate-css.ts`, `tailwind-css.ts`,
  `parse-css-input.ts`). They write to `document.documentElement`
  or generate CSS strings. Mobile reads the same store and exposes
  resolved tokens through context — see
  [NEW-PACKAGES.md](./NEW-PACKAGES.md#starter-theme-tokens).

## Reused — SDUI **after** a package split (blocker)

`@nube/starter-ui-sdui-react` today ships a single `"."` export
whose `src/index.ts` ends with `export * from "./renderer/index.js"`.
Every `render-*.tsx` imports `@nube/starter-ui-kit` (Radix +
Tailwind). Importing the root barrel from mobile pulls Radix into
the RN bundle, which defeats the whole reuse story.

**Mobile blocks on a package split that adds a headless entry**
(see [NEW-PACKAGES.md §Precondition](./NEW-PACKAGES.md#precondition--sdui-react-package-split)).
Proposed shape:

```
@nube/starter-ui-sdui-react/headless   ← SduiPage, SduiProvider,
                                          hooks, transport,
                                          registerRenderer/lookupRenderer/listRenderers
@nube/starter-ui-sdui-react             ← today's root; web renderers + headless re-export
```

Until that split lands, mobile cannot import from
`starter-ui-sdui-react` at all.

**Allowed (after split):** the `/headless` subpath.
**Forbidden:** the root `"."` export and anything under `/renderer/*`.

The registry must live in `/headless` so both consumers share one
module instance — see
[NEW-PACKAGES.md](./NEW-PACKAGES.md#starter-ui-sdui-native) for the
single-instance hazard.

## Web fixups required

This plan depends on three upstream changes in `packages/` that
must land **before or alongside** mobile work:

1. `starter-ui-sdui-react` — add a `./headless` subpath export
   per the section above.
2. `starter-ui-core` — either expose the sub-paths the mobile
   import-lint rule names (e.g. `./theme-editor/utils/color-converter`)
   as additional `exports` entries, or accept that mobile imports
   them as named re-exports from `./theme-editor` and update the
   lint rule accordingly. This file uses the named-export route.
3. `starter-ui-core/preferences` — split the React `PreferencesProvider`
   from the types/formatters/store so mobile can import the latter
   without the former.

If these don't land, REUSE.md is unenforceable and the import-lint
rule in [APP-SHELL.md](./APP-SHELL.md#import-lint) is the only
thing stopping a DOM dep from landing in the RN bundle.

## Reused — JSON catalogs

- [`rubix/frontend/src/i18n/en.json`](../../../frontend/src/i18n/en.json)
- [`rubix/frontend/src/i18n/es.json`](../../../frontend/src/i18n/es.json)

Imported directly. If a string is mobile-only, add it to the same
catalog under a `mobile.*` key — do not fork.

## Explicitly NOT reused

If you find yourself wanting one of these, stop and read the rest
of this file again.

| Package | Why excluded |
|---|---|
| `@nube/starter-ui-kit` | Radix + Tailwind. Replaced by `starter-ui-kit-native`. |
| `@nube/starter-ui-flow` | `@xyflow/react`; not a phone surface. |
| `@nube/starter-ui-export` | `html2canvas` + `jspdf`; DOM-only. |
| `@nube/starter-ui-authz` | Admin panels; not a phone surface. |
| `@nube/starter-ui-ai-builder` | Web split-pane composition. |
| `@nube/starter-sdui-react` | Older D2 renderer set. Same web-only constraint as the current renderers, and we are not maintaining two on mobile. |
| `@nube/starter-ui-dashboard` | Replaced by `starter-ui-dashboard-native` (same prop API, RN primitives). |
| All of `rubix/frontend/src/{routes,components,styles,lib/flow-*}` | App-local shell, Tailwind CSS, flow editor wiring. |
