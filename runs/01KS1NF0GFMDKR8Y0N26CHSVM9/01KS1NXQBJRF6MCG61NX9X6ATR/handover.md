## Done

- Added two new singletons to `ExtensionHostManager` table via the notes host's `createExtensionHost`: `@nube/starter-ui-core/preferences` (instance = `PreferencesContext`) and `@nube/starter-ui-core/i18n` (instance = `IntlContext`), each pinned at version `1.0.0` (`UI_CORE_PREFERENCES_VERSION` / `UI_CORE_I18N_VERSION`).
- Exported `PreferencesContext` and `IntlContext` (plus `IntlContextValue` type) from `@nube/starter-ui-core/preferences` and `…/i18n`.
- Added `parseMinor`, `SingletonMinorDrift`, and `SINGLETON_*` id constants to `starter-extensions/packages/starter-ext-ui/src/singletons.ts`.
- Added `ExtensionHostTelemetryEvent` + `ExtensionHostTelemetrySink` to host-manager and wired emission: `extension.singleton_mismatch` (error) fires before the existing `SingletonMismatchError` throw; `extension.singleton_minor_drift` (warn) fires on successful registration when the extension is behind on minor (same major). Patch drift silent; host-ahead-on-minor not flagged. Sink throws are caught and console-warned.
- Notes `extension-host.ts` provides a default console-based telemetry sink and accepts an override via `BootstrapInput.telemetry`.
- Tests: extended `host-manager.test.ts` with the Stage-2 telemetry block (instance pass-through, mismatch, minor drift, silent patch, host-behind no-op, sink throw swallow) and extended `singletons.test.ts` with `parseMinor` coverage. All 22 ext-ui tests + 4 notes-frontend tests green; both ui-core and ext-ui typecheck clean.
- Committed as `c72500f` on `codeless/notes-prefs-i18n`.

## Next

- Stage 3: ship `@nube/starter-ext-sdk-ts` hooks (`useHostPrefs`, `useHostTranslate`, `useHostFormatters`), the `MockHostProvider` test helper, and `MessageKey` codegen. The hooks should read `handle.singletons[SINGLETON_UI_CORE_PREFERENCES]` (a React Context) and `useContext` against it.

## What you need to know

- The singleton instance for the two new entries is the React Context object itself, not the `{ value, setPreferences }` value — Stage 3 hooks `useContext(handle.singletons["@nube/starter-ui-core/preferences"])` against the host's instance.
- Version policy lives in `examples/notes/frontend/src/extension-host.ts` (`UI_CORE_PREFERENCES_VERSION = "1.0.0"`); bumping it is the lever for refusal-on-major / warn-on-minor without touching the ui-core package version.
- Telemetry event names are public contract (per user-pref.md § Telemetry). Field shape: `{ kind, severity, extensionId, reasons|drifts }`.
- `checkMinorDrift` is intentionally one-sided: only the extension-behind case is flagged; extension-ahead is "host needs updating," not an extension issue.
- `extension.singleton_mismatch` is emitted *before* the `SingletonMismatchError` throws so dashboards see the refusal even if the caller swallows the exception (matches user-pref.md wording about console logs no operator reads).

## Open questions

- (none)
