## Done

- Added packages/starter-ui-core/src/i18n/ (types, fetcher with permanent fingerprinted-URL cache, IntlProvider via RawIntlProvider+createIntl, useTranslate with typed AppMessageKeys hook, useIntlContext, index re-exports).
- Added packages/starter-ui-core/src/preferences/SettingsPage.tsx with selectors for every ResolvedPreferences field, diff-based PATCH submit, injectable onToast, useTranslate-driven copy at starter.settings.* keys.
- Added react-intl@^7 dep (v10 needs React 19; workspace is React 18).
- Wired ./i18n into package.json#exports; re-exported SettingsPage from src/preferences/index.ts and from src/i18n/index.ts.
- Documented consumer mounting in crates/starter-auth-users/README.md and packages/starter-ui-core/README.md.
- Tests: 3 useTranslate (active locale, en fallback, verbatim id), 2 IntlProvider (initial load, remount on language switch), 1 SettingsPage happy path. pnpm test → 75 passed; pnpm typecheck → clean.
- Committed as "stage 18 — Phase 4 IntlProvider wiring + Settings page".

## Next

- Stage 19 of 22 (per the job plan) — Phase 5 begins (scope-limited diagnostics rewriter on starter-i18n). Fresh session picks it up.

## What you need to know

- react-intl 7 was pinned because v10 transitively requires React 19 / @types/react 19 which conflict with the workspace's React 18. Even at v7 the @types/react bleed from react-intl forced two narrow `as unknown as` casts (RawIntlProvider component type and useTranslate return type) — purely type-level, runtime is untouched.
- IntlProvider uses RawIntlProvider + createIntl rather than react-intl's class-based <IntlProvider> to bypass a @types/react 18-vs-19 JSX-element-type clash. It re-keys on activeLanguage so react-intl's compiled-MessageFormat cache resets cleanly on language switch.
- The catalog cache (loadCatalogCached) and the manifest promise are module-level singletons; test helpers `_resetCatalogCacheForTesting` / `_resetManifestCacheForTesting` are exported for vitest cleanup.
- SettingsPage's `useSyncDraft` only seeds the draft once from upstream — intentional pragmatic dirty-bit; documented inline. Revisit if multi-tab live sync becomes a requirement.
- starter-auth-users is a Rust crate, so "wire the Settings page into starter-auth-users' account page surface" was satisfied by documenting the consumer mount pattern in that crate's README (no React surface lives there to wire directly).

## Open questions

- (none)
