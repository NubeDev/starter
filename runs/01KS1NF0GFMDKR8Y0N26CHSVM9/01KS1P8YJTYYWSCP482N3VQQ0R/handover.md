## Done

- Added `useHostPrefs` / `useHostTranslate` / `useHostFormatters` to `@nube/starter-ext-sdk-ts` (read host singletons via internal `HostBindingsProvider`; auto-prefixed bare keys per D-NP.3).
- `registerExtensionContributions` now wraps every contributed component with `<HostBindingsProvider bindings={{extensionId, singletons}}>` so the hooks resolve without per-extension wiring; new wrapper has a `HostBindings(<id>:<name>)` displayName.
- Generated `PlatformMessageKey` union (82 keys) from `crates/starter-i18n/catalogs/starter/en.json` via `scripts/gen-message-keys.mjs` (exposed as `pnpm --filter @nube/starter-ext-sdk-ts gen-keys`). Public `MessageKey` = `PlatformMessageKey | keyof ExtensionMessageKey | (string & {})`; extension authors augment `ExtensionMessageKey`.
- Shipped `@nube/starter-ext-sdk-ts/testing` with `MockHostProvider` (fresh per-mount Context objects, mock IntlShape with naive `{var}` substitution, optional `intl` override).
- Wrote `src/extension-prefs-singleton.test.tsx` (5 cases: en-AU happy path, missing-key id fallback, missing-provider throw, missing-singleton throw, registration-wrap invariant). Updated `register.test.ts` to assert the wrapping rather than identity.
- Modified `packages/starter-ui-core/src/i18n/provider.tsx` to expose react-intl's `IntlShape` on `IntlContextValue.intl` so the SDK can call `formatMessage` against the host's catalog even when the extension bundles its own react-intl.
- Added jsdom + @testing-library/react + react-dom to SDK devDeps; switched its vitest environment to jsdom.
- Committed as "Stage 3 — SDK prefs/i18n hooks + MockHostProvider + MessageKey codegen" on `codeless/notes-prefs-i18n`.

## Next

- Stage 4: rewrite `extensions/com.nube.hello/ui/remoteEntry.js` to declare the two new singletons and render localised + AU-formatted lines using the Stage 3 hooks.

## What you need to know

- The SDK keeps the dep arrow honest: it mirrors `ResolvedPreferences` (and a 100-line slice of `formatters.ts` + the unit conversion table) locally in `prefs-types.ts` and `formatters.ts` rather than importing from `@nube/starter-ui-core`. Both copies update in the same PR if the Rust DTOs change.
- `HostIntlShape` in the SDK is a duck-typed `{ formatMessage(d, v?): string }` — keeps `react-intl` out of the SDK's type graph.
- The host wires `IntlContextValue.intl` from the live `createIntl(...)`; the field is typed `unknown` in ui-core so consumers without react-intl in scope (the SDK) can narrow as needed.
- `registerExtensionContributions` is now `.tsx` (renamed from `.ts`) — JSX inference was the simplest fix for the `children`-prop typing.
- `extensions/hello-ui` example still emits a peer-dep warning on `@types/react-dom@19` vs `@types/react@18`; pre-existing, unrelated to this stage.

## Open questions

- (none)
