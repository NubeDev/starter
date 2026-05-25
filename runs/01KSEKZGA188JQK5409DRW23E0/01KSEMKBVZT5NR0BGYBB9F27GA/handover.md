## Done

- Extracted DOM-free symbols from `packages/starter-ui-core/src/preferences/provider.tsx` into new `provider-core.ts` (PreferencesContext, usePreferences, DEFAULT_WORKSPACE, PREFERENCES_BROADCAST_CHANNEL, PreferencesBroadcastMessage, SR_ONLY_STYLE, buildLanguageAnnouncement, LANG_CHANGED_PHRASE, fetchMyPreferences, patchMyPreferences, plus a new pure `isRtlLanguage` helper).
- `provider.tsx` now imports from `provider-core.js`, re-exports every core symbol for backwards-compat, and keeps `document.documentElement.lang`/`.dir` writes + aria-live `<div>` exactly as before; web `PreferencesProvider` mount/toggle behaviour unchanged. `i18n/provider.tsx` (sole external `./provider.js` importer) keeps working.
- Added `packages/starter-ui-core/src/theme-editor/WEB-ONLY.md` listing web-only theme-editor named exports (localStorageThemeTransport, applyTheme, applyPreferences, generateCss, tailwindCss, parseCssInput) and confirming the guarded layout-preferences matchMedia helpers are RN-safe.
- Audited the `theme-editor/index.ts` barrel: types, defaults, presets, store, layout-preferences (+ store), color-converter, contrast-checker, all hooks are reachable as named re-exports (already via `export *`); no additions needed.
- Added `packages/starter-ui-core/src/theme-editor/__tests__/layout-preferences.node.test.ts` pinning the L81/L92/L104 `typeof window === "undefined" || typeof window.matchMedia !== "function"` guards by stripping `window` / `window.matchMedia` and asserting `resolveMode("system") === "light"` and the subscribe helpers return no-op unsubscribers.
- `pnpm --filter @nube/starter-ui-core typecheck` passes. Test run: 102 pass / 2 fail; the two failures (`auth/auth.test.tsx`) are pre-existing — reproduced on a clean `git stash` of this stage's changes.
- Committed as `904019e` on `codeless/mobile-chassis`.

## Next

- (none) — stage 2 only.

## What you need to know

- No `package.json#exports` entries were added; the `./preferences` and `./theme-editor` subpaths keep the same surface. RN consumers in later stages can either deep-import `@nube/starter-ui-core/preferences/provider-core` once `provider-core.ts` is added to a future subpath, or (recommended) re-export it from the upcoming `@nube/starter-ui-kit-native` native barrel.
- The new `isRtlLanguage(language: string)` helper in `provider-core.ts` is intentionally **not** re-exported from `./preferences/index.ts` yet — additive-only rule. Native code can grab it through the same future barrel.
- `WEB-ONLY.md` is the authoritative exclude-list for the native theme module factory work in later stages.
- Vitest config is jsdom-global; the node-shape test simulates Hermes by stripping `window` rather than using a per-file environment pragma. `afterEach` restores the original `window` so other jsdom tests keep passing.

## Open questions

- (none)
