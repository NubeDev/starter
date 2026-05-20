## Done

- Rewrote examples/notes/extensions/com.nube.hello/ui/remoteEntry.js: factory declares react + @nube/starter-ui-core/preferences + @nube/starter-ui-core/i18n singletons (versions pinned to 1.0.0), reads PrefsContext + IntlContext off handle.singletons, loud-fails when either is missing.
- HelloPanel now reads prefs + intl via React.useContext, renders four Stage-4 surfaces: greeting (com.nube.hello.greeting), unread plural (com.nube.hello.unread), formatDate(Date.now()) against prefs, and formatQuantity 22.44 °C → preferred temperature unit.
- Inlined minimum subset of SDK formatters.ts (affine temperature table, dateOptions, numberLocaleChain) so the hand-written ESM bundle byte-matches what useHostFormatters would produce.
- Each Stage-4 surface tagged with data-testid (`hello-greeting`, `hello-unread`, `hello-date`, `hello-temperature`) for Stage-6 Playwright pinning.
- Existing greet REST round-trip + status badge preserved.
- node --check passes; committed as 73841eb with subject starting "Stage 4 —".

## Next

- Stage 5: add `requires: [@nube/starter-ui-core/preferences, @nube/starter-ui-core/i18n]` and `contributes.i18n.catalogs` to extensions/com.nube.hello/block.yaml, ship i18n/{en,es}.json carrying `hello.greeting` + `hello.unread` ICU plural, wire host registerExtensionMessages lazy per-language load + collision telemetry, add CI check:i18n gate.

## What you need to know

- remoteEntry.js is still hand-written ESM with no bundler — cannot import @nube/starter-ext-sdk-ts. The Stage-5 catalog wiring will be host-side (host fetches /extensions/<id>/i18n/<lang>.json and merges into IntlProvider). The panel itself doesn't need to import anything new.
- Catalog keys are fully-qualified inside the panel (`com.nube.hello.greeting`, not bare `greeting`), matching D-NP.3 — the SDK's auto-prefix only fires for hooks; this hand-written bundle uses intl.formatMessage directly.
- Until Stage 5 catalogs land, intl.formatMessage returns the id verbatim — extension-prefs-singleton.test.tsx already covers that fallback behaviour; the production smoke depends on Stage 5.
- Singleton versions declared at "1.0.0" to match UI_CORE_PREFERENCES_VERSION / UI_CORE_I18N_VERSION pinned in examples/notes/frontend/src/extension-host.ts.

## Open questions

- (none)
