## Done

- Extended `@nube/starter-ui-core` `<PreferencesProvider>` with a `fallback` prop (loading contract — children only mount once prefs resolve) and a `document.documentElement.lang` side-effect tied to `prefs.language`.
- Added `examples/notes/frontend/src/prefs-host.tsx` exposing `<PrefsHostShell>` (QueryClientProvider → PreferencesProvider → IntlProvider) and `<PrefsProbe>` (renders one fixture date + one fixture temperature against resolved prefs).
- Wired `<PrefsHostShell>` inside the auth boundary in `examples/notes/frontend/src/app.tsx`, mounted `<PrefsProbe>` in the header, added a "Settings" tab that renders `<SettingsPage />` from `@nube/starter-ui-core/preferences`, and a branded `PrefsLoadingSkeleton` fallback.
- Enabled `starter-prefs` (routes + sqlite) and `starter-i18n` (routes) cargo features on the notes binary in `examples/notes/Cargo.toml`.
- Authored `examples/notes/frontend/src/prefs-host.test.tsx`: stubs `/v1/me/preferences` + `/v1/i18n/manifest`, asserts `22/04/2026`, `72.4 °F`, `document.documentElement.lang === "en-AU"`, and the loading-contract gate. Test passes; `pnpm typecheck` clean in both `examples/notes/frontend` and `packages/starter-ui-core`; `cargo check -p starter-notes` passes.
- Committed as `Stage 1 — Notes host adopts PreferencesProvider + IntlProvider` (c697b4b).

## Next

- Stage 2: extend `ExtensionHostManager`'s singleton table with `@nube/starter-ui-core/preferences` and `@nube/starter-ui-core/i18n`, with the R11 handshake (major-mismatch → `Failed` lifecycle + telemetry; minor-drift warn).

## What you need to know

- `PrefsHostShell` constructs its own `QueryClient` by default but accepts an override so the rest of the notes app (or tests) can share a cache.
- The added `<PreferencesProvider>` loading gate (children blocked until `query.data != null`) is observed by the existing `provider.test.tsx` — it still passes because react-query keeps `data` defined across refetches after the first success.
- `IntlProvider` mounts inside `PreferencesProvider` so it reads `prefs.language` directly; the smoke test serves an empty manifest because `PrefsProbe` formats via `Intl.*` (formatters), not via `react-intl` — translated strings land in Stage 4 once `useHostTranslate` exists.
- The notes server now depends on `starter-prefs` and `starter-i18n` crates, but no route is wired into the axum app yet — the deliverable named only the Cargo features. Wiring the routers will come when a later stage exercises the live HTTP surface end-to-end.
- Cargo.lock was regenerated automatically by `cargo check`.

## Open questions

- (none)
