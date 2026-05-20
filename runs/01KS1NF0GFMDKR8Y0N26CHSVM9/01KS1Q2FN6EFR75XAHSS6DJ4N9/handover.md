## Done

- reviewed Stage-2 (singleton channel) + Stage-3 (SDK hook surface) diffs against rulebook Layer-1 invariants
- confirmed R1/R2/R4/R5 hold and wire formats are untouched
- confirmed SDK hook signatures, MessageKey codegen shape, and MockHostProvider API are consumable by the upcoming com.nube.hello rewrite
- PASS: Layer-1 invariants hold and the SDK contracts are ready to be depended on by Stages 4–6

## Next

- Stage 5 (next session): execute Stage 4 of examples/notes/user-pref.md — rewrite extensions/com.nube.hello/ui/remoteEntry.js to declare `@nube/starter-ui-core/preferences` and `…/i18n` in its `singletons:`, thread the resolved handle through `registerExtensionContributions`, and render greeting + plural + date + BBQ-quantity surfaces via `useHostTranslate`/`useHostFormatters`. Catalog files land at `extensions/com.nube.hello/i18n/{en,es}.json`.

## What you need to know

- Singleton ids are the package+subpath constants in `starter-extensions/packages/starter-ext-sdk-ts/src/singleton-keys.ts` (mirror of `starter-ext-ui/src/singletons.ts`) — never invent shorthand like `"prefs"`.
- `registerExtensionContributions` already wraps registered components in `<HostBindingsProvider>`; the panel itself just calls hooks, no manual context plumbing.
- Bare keys passed to `useHostTranslate` are auto-prefixed with `handle.id` (D-NP.3); fully-qualified keys pass through verbatim — used in the catalog under `com.nube.hello.*`.
- Platform telemetry event names are frozen contracts: `extension.singleton_mismatch`, `extension.singleton_minor_drift` (and the later `extension.catalog_key_collision`). Do not rename.

## Open questions

- Stage 5 will need to settle the html-lang spelling after a language flip (`es` vs `es-AU`) — the Playwright spec at Stage 6 pins whichever the resolver actually emits.

PASS: Stage 2 + Stage 3 preserve R1 dep direction, R2 single (React-Context) transport, R4/R5 trust boundary with telemetry-backed major-mismatch refusal, and leave the i18n catalog + prefs wire formats unmodified.
