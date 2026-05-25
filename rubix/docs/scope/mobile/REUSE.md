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

## Reused — subpath imports only

`@nube/starter-ui-core` ships subpath exports; mobile imports the
DOM-free subset and never the rest. The subset and the forbidden
list are enforced by the import-lint rule in
[APP-SHELL.md](./APP-SHELL.md#import-lint).

**Allowed subpaths:**

- `@nube/starter-ui-core/auth` — `AuthProvider`, `useAuth`,
  pluggable `AuthStrategy`. Mobile picks the **token** strategy,
  not the cookie one.
- `@nube/starter-ui-core/query` — `starterQueryKey` namespacing.
- `@nube/starter-ui-core/i18n` — locale store, manifest loader,
  fetcher, fallback. `react-intl` runs on RN without changes.
- `@nube/starter-ui-core/preferences` — types, formatters, units.
  Intl APIs are in Hermes/JSC.
- `@nube/starter-ui-core/theme-editor` — `types`, `defaults`,
  `presets`, `store`, `layout-preferences*`, `utils/color-converter`,
  `utils/contrast-checker`. The **state model** only.

**Forbidden subpaths:**

- `@nube/starter-ui-core/layout` — `cookies.ts` reads
  `document.cookie`; `use-mobile.ts` uses `matchMedia`. Mobile
  replaces both: bearer-token auth via AsyncStorage, breakpoints
  via `useWindowDimensions()`.
- The `apply-*` helpers in `theme-editor/utils/` (`apply-theme.ts`,
  `apply-preferences.ts`, `generate-css.ts`, `tailwind-css.ts`,
  `parse-css-input.ts`). They write to `document.documentElement`
  or generate CSS strings. Mobile reads the same store and exposes
  resolved tokens through context — see
  [NEW-PACKAGES.md](./NEW-PACKAGES.md#starter-theme-tokens).

## Reused — subpath imports only, SDUI

`@nube/starter-ui-sdui-react` is split between web-only renderers
and a portable orchestration core.

**Allowed:**

- `@nube/starter-ui-sdui-react` (root) — `SduiPage`, `SduiProvider`,
  `PageStateProvider`, `useSduiResolve`, `useSduiAction`,
  `useSduiSubscriptions`.
- `@nube/starter-ui-sdui-react/transport` — `createHttpSduiTransport`.

**Forbidden:**

- `@nube/starter-ui-sdui-react/renderer` — the registry plus every
  `render-*.tsx`. They import `@nube/starter-ui-kit`. Mobile
  registers its own renderers from
  [`@nube/starter-ui-sdui-native`](./NEW-PACKAGES.md#starter-ui-sdui-native).

The registry mechanism (`registerRenderer` / `lookupRenderer` /
`listRenderers`) is the integration seam. Mobile calls the same
registry from a different set of components.

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
