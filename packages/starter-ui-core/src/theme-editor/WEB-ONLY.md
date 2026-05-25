# `theme-editor` — web-only surface

This module ships as part of `@nube/starter-ui-core` and is reached
via the `./theme-editor` subpath export. Most of it is pure data
(types, defaults, presets, colour utilities) and is safe to import
from any JavaScript runtime, including React-Native / Hermes.

A handful of named exports **touch browser-only globals** (`window`,
`document`, `localStorage`, `matchMedia`, `MediaQueryList`,
`CSSStyleDeclaration`, the DOM at large) and must not be imported
from React-Native code. They are listed here verbatim so platform
splits (the upcoming `@nube/starter-ui-kit-native` consumer) can
mechanically exclude them from the native bundle graph.

## Web-only named exports (do NOT import from React-Native)

| Symbol                              | File                                  | Why it's web-only                                                         |
| ----------------------------------- | ------------------------------------- | ------------------------------------------------------------------------- |
| `localStorageThemeTransport`        | `transport.ts` (L91, uses L96/L106)   | Reads/writes `window.localStorage`.                                       |
| `applyTheme`                        | `utils/apply-theme.ts`                | Writes CSS custom properties onto `document.documentElement`.             |
| `applyPreferences`                  | `utils/apply-preferences.ts`          | Toggles `data-*` attributes and class names on `document.documentElement`. |
| `generateCss`                       | `utils/generate-css.ts`               | Emits a CSS stylesheet string consumed by the web SPA's `<style>` tag.    |
| `tailwindCss` (and re-exports)      | `utils/tailwind-css.ts`               | Tailwind preset wiring — only meaningful inside a PostCSS/Tailwind build. |
| `parseCssInput`                     | `utils/parse-css-input.ts`            | Parses a pasted-in `:root { --foo: ... }` CSS block (web theme-editor).   |

The `subscribePrefersDark`, `subscribePrefersReducedMotion` and
`resolveMode` helpers in `layout-preferences.ts` (lines 81, 92, 104)
also touch `window.matchMedia`, but they ship guarded
(`typeof window === "undefined" || typeof window.matchMedia !== "function"`)
and return a no-op on RN/Hermes — they're safe to import from
native code, they just won't fire.

## Universal (safe everywhere) named exports

Everything else re-exported from `./theme-editor/index.ts` is pure
data or platform-agnostic logic:

- `./types` — every theme type (`ThemeDocument`, `ThemeToken`, …).
- `./defaults` — built-in default theme values.
- `./presets` — packaged colour/typography presets.
- `./store` — the editor's zustand store (state lives in memory).
- `./layout-preferences` — `ModePreference`/`ResolvedMode`/density
  enums + `resolveMode` (guarded), `subscribePrefersDark` (guarded),
  `subscribePrefersReducedMotion` (guarded).
- `./layout-preferences-store` — zustand store for layout prefs.
- `./utils/color-converter` — pure colour-space conversion.
- `./utils/contrast-checker` — pure WCAG contrast math.
- `./hooks/use-theme-editor`, `./hooks/use-theme-presets` — React
  hooks; safe in any React renderer (DOM or Native) because they
  only read/write the zustand stores listed above.

When the native UI kit lands (`@nube/starter-ui-kit-native`), the
native theme transport will be a separate factory (e.g. an
`AsyncStorage`-backed transport) living in that package — this file
is the authoritative list of what it must avoid re-exporting.
