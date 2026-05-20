## Done

- Added examples/notes/frontend/e2e/au-operator.spec.ts covering: au-bbq-operator seed (en-AU/metric/fahrenheit/en), AU date + °F render on host chrome (PrefsProbe) and HelloPanel, language flip to es with no reload, <html lang> flip, second-tab BroadcastChannel propagation (same browser context), temperature_unit °F→°C flip on both tabs, and axe-core scans before/after (zero violations expected).
- Added @axe-core/playwright as a devDependency of examples/notes/frontend and ran pnpm install (lockfile updated).
- `tsc --noEmit -p tsconfig.json` is clean.
- Committed as 92d0d2c with the required stage-title prefix.

## Next

- Stage 7 (workflow) — ship the production cross-cuts the spec assumes are wired: locale fallback chain (D-NP.6), i18n.message_missing + i18n.locale_fallback telemetry, BroadcastChannel("starter-prefs") multi-tab in PreferencesProvider, dev catalog watcher in the Vite plugin, render-budget Vitest, aria-live language-change announcer, and the three doc deliverables.

## What you need to know

- The spec PATCHes /v1/me/preferences directly against http://127.0.0.1:8080 (the Rust backend started by global-setup) before navigation; it does not rely on a pre-seeded fixture user.
- HelloPanel test ids already exist (hello-greeting / hello-unread / hello-date / hello-temperature) from Stage 4; host chrome test ids prefs-probe-date / prefs-probe-temp from Stage 1.
- BroadcastChannel multi-tab is NOT yet wired (Stage 7); running the spec today will fail tab-2 propagation assertions until that ships. That's expected — the spec is the merge gate, per the scope ("the test is the spec").
- axe-core color-contrast is disabled — contrast failures live in theme tokens, not in this PR's surface.
- Language match regex is `^es(-AU)?$` per the scope's "pin whichever the resolver actually emits"; Stage 7 should confirm which value lands and the regex stays compatible either way.

## Open questions

- (none)
