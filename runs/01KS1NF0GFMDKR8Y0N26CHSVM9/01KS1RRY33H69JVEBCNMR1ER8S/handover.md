## Done

- Locale fallback chain (D-NP.6): packages/starter-ui-core/src/i18n/locale-fallback.{ts,test.ts} (pure left-truncating BCP-47, en floor, case-insensitive, 9 unit tests). IntlProvider rewired to use it; emits `i18n.locale_fallback` once per (requested,picked) per session.
- i18n telemetry sink: packages/starter-ui-core/src/i18n/telemetry.ts (`setI18nTelemetry`, `emitI18nTelemetry`). `useTranslate` fires `i18n.message_missing` when react-intl returns the id verbatim; SDK `useHostTranslate` fires via host-supplied `reportMissingKey` on IntlContextValue (keeps SDK dep arrow clean).
- Prefs telemetry sink: packages/starter-ui-core/src/preferences/telemetry.ts (`prefs.broadcast_dropped`).
- BroadcastChannel("starter-prefs") in PreferencesProvider: posts on successful setPreferences, listener applies the patch optimistically + invalidates the query. Channel name exported as PREFERENCES_BROADCAST_CHANNEL (frozen per D-NP.9).
- A11y: PreferencesProvider sets `<html lang>` + `<html dir>` and renders a polite `aria-live` region announcing language flips in the new language (`"Idioma cambiado a Español"`, etc.).
- Render-budget: examples/notes/frontend/src/prefs-render-budget.test.tsx asserts three sibling consumers render exactly twice across a language flip. Required funnelling setPreferences through a ref so react-query mutation state churn does not invalidate the context value.
- Dev catalog watcher: examples/notes/frontend/vite-plugin-i18n-watcher.ts + wired into vite.config.ts. Dev-only (`apply: "serve"`); triggers HMR full-reload on `extensions/**/i18n/*.json` changes.
- Notes app installs both telemetry sinks at boot (examples/notes/frontend/src/app.tsx).
- Three docs: DOCS/extensions/guides/i18n.md, DOCS/user/guides/prefs-in-extensions.md, examples/notes/extensions/com.nube.hello/README.md (mirror also written into docs/README.md so the manifest's description_file still resolves).
- All checks green: `pnpm -w build`, `pnpm --filter @nube/starter-ui-core test` (84/84), `pnpm --filter @nube/starter-ext-sdk-ts test` (6/6), `pnpm --filter starter-notes-frontend test` (8/8 including new render-budget), `pnpm -w run check:i18n`, `cargo check -p starter-notes`.
- Committed `0884770` on `codeless/notes-prefs-i18n`, pushed.

## Next

- (none) — Stage 9 of 9; final stage of the job.

## What you need to know

- The six telemetry event names are kept verbatim (frozen contract per SCOPE.md). Sink installation is process-wide via `setI18nTelemetry` / `setPreferencesTelemetry`; the notes host wires both at app mount.
- The SDK does NOT import `@nube/starter-ui-core`. Missing-key routing for extensions goes through an optional `reportMissingKey` field on `HostIntlContextValue`; older hosts that don't supply it simply skip the emit.
- The dev catalog watcher uses `full-reload` rather than surgical HMR — pragmatic; the prod merge path already handles language flips correctly.
- `starter-extensions/examples/notes` package failing `pnpm -w test` ("No test files found, exiting with code 1") is pre-existing (reproduces on master before my edits) — not introduced by this stage.
- Render budget fix relied on stabilising `setPreferences` via a ref to `mutation.mutateAsync`; without this, react-query's mutation state transitions (idle → pending → success) cause the memoised context value to change 2–3 times per flip and blow the one-render-per-consumer budget.

## Open questions

- (none)
