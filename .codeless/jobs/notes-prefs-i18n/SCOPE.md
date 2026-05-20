# Scope — notes-prefs-i18n

> **The full design lives in
> [`examples/notes/user-pref.md`](../../../examples/notes/user-pref.md).**
> This file is the per-job brief. When this brief disagrees with the
> design doc, the design doc wins. When the design doc disagrees with
> the platform SCOPEs — [`DOCS/user/scope/SCOPE.md`](../../../DOCS/user/scope/SCOPE.md)
> (R1–R9) and [`DOCS/extensions/scope/SCOPE.md`](../../../DOCS/extensions/scope/SCOPE.md)
> (R1–R13) — the platform SCOPEs win.

## Goal

Wire the existing prefs + i18n surface in `@nube/starter-ui-core`
(`PreferencesProvider`, `IntlProvider`, `SettingsPage`, formatters)
into [`examples/notes`](../../../examples/notes) end-to-end through one
Module-Federation extension (`com.nube.hello`). Prove the contract:

1. An Australian operator (`en-AU`, `Australia/Brisbane`, metric +
   `temperature_unit: F`) sees AU dates and BBQ °F across host
   chrome **and** the federated `HelloPanel`.
2. Flipping language to `es` updates both surfaces in one render
   with no page reload.
3. The same flip propagates to a second open tab within one
   animation frame via `BroadcastChannel("starter-prefs")`.
4. Ship the production-hardening cross-cuts in the same PR so the
   contracts (singleton ids, semver promise, telemetry events,
   catalog packaging, a11y, multi-tab) are the reference every
   starter-based product copies.

## In scope

- Host wiring: `PreferencesProvider` + `IntlProvider` + `SettingsPage`
  in [`examples/notes/frontend/src/app.tsx`](../../../examples/notes/frontend/src/app.tsx);
  `document.documentElement.lang` owned by the provider.
- Singleton channel: `@nube/starter-ui-core/preferences` +
  `@nube/starter-ui-core/i18n` added to the
  [`extension-host.ts`](../../../examples/notes/frontend/src/extension-host.ts)
  registry; major-mismatch refusal + minor-drift warn.
- SDK hooks: `useHostPrefs`, `useHostTranslate`, `useHostFormatters`
  in `@nube/starter-ext-sdk-ts`; `MessageKey` codegen union;
  `MockHostProvider` test helper.
- `com.nube.hello` panel rewrite using the hooks (greeting from
  catalog, ICU plural, AU date, BBQ temp).
- `block.yaml` `requires` + `contributes.i18n.catalogs`; namespaced
  merge; lazy per-locale fetch; `pnpm -w run check:i18n` CI gate;
  collision warning.
- Playwright `au-operator.spec.ts` covering single-tab + multi-tab
  + a11y.
- Production cross-cuts: locale fallback chain (D-NP.6), missing-key
  telemetry, BroadcastChannel multi-tab, dev catalog watcher,
  render-budget test, aria-live announcement.
- Three docs: [`DOCS/extensions/guides/i18n.md`](../../../DOCS/extensions/guides/i18n.md),
  [`DOCS/user/guides/prefs-in-extensions.md`](../../../DOCS/user/guides/prefs-in-extensions.md),
  hello extension README update.

## Out of scope

- Rust `Ctx::prefs()` / `Ctx::locale()` for backend extensions —
  separate extensions-addendum scope.
- SSR of federated panels.
- Per-extension preference overrides (extensions read, never write
  their own prefs).
- WASM / process extension flavours — handshake is the same but
  the demo stays builtin.
- RTL catalogs (provider sets `dir` for known RTL languages but no
  RTL catalog ships).
- Server-side translation of emails / PDFs.
- Server-side cross-device prefs sync (BroadcastChannel covers the
  same-browser case only; SSE/long-poll is a v2 — but reserve
  `GET /v1/me/preferences/stream` returning 501 in Stage 7).

## Constraints

- TypeScript strict, ESM only. No new top-level deps in
  `examples/notes/frontend/package.json`.
- `pnpm -w build`, `pnpm -w test`, `pnpm -w run check:i18n` (new)
  all green.
- `cargo check -p starter-notes` + `cargo test -p starter-notes`
  stay green; no new Rust deps in `notes/Cargo.toml` beyond
  `starter-prefs` + `starter-i18n`.
- Manifest stays `v: 1` (D-NP.4). No schema bump.
- Initial JS payload for `examples/notes/frontend` grows by **≤ 8 KB
  gzipped** over master; measured by `pnpm -w run analyze:bundle`.
- Render budget: a single `setPreferences` call re-renders each
  context consumer exactly once (Stage 7 Vitest).
- A11y: axe-core in the Playwright spec reports zero new violations
  vs master.
- Singleton ids match the package + subpath an extension would
  `import` (D-NP.1).
- Hooks live in `@nube/starter-ext-sdk-ts`, not `ui-core` (D-NP.2).
- Catalog format is flat-keyed ICU MessageFormat (D-NP.7).
- Catalogs lazy-load per active language (D-NP.8).
- Multi-tab via `BroadcastChannel("starter-prefs")` only; no
  server-side push (D-NP.9).
- Semver promise per D-NP.10: major refuses, minor warns, patch
  silent. SDK package versions in lockstep with the singleton major.
- Telemetry event names are stable and frozen on merge:
  `extension.singleton_mismatch`, `extension.singleton_minor_drift`,
  `extension.catalog_key_collision`, `i18n.locale_fallback`,
  `i18n.message_missing`, `prefs.broadcast_dropped`.

## Smoke tests (merge gates)

All run against the in-process sqlite store + notes shell stub
client. No real LLM, no real OAuth, no real Postgres.

1. `prefs-host.test.tsx` (Stage 1) — host renders AU date + BBQ
   °F + asserts `<html lang>` is set.
2. `extension-prefs-singleton.test.tsx` (Stage 3) — fake extension
   under fake host reads the three hooks correctly.
3. `extension-catalog-merge.test.tsx` (Stage 5) — namespaced merge,
   collision warning fires.
4. `locale-fallback.test.ts` (Stage 7) — pure resolver: `es-MX → es`,
   `pt-BR → pt → en`, unknown → `en`.
5. `prefs-render-budget.test.tsx` (Stage 7) — three siblings re-render
   exactly twice across a language flip.
6. `au-operator.spec.ts` (Stage 6) — Playwright e2e: language flip,
   unit flip, second-tab BroadcastChannel, `<html lang>` flip,
   axe-core clean.

## Open questions (resolve in stage 1)

1. **Where the `<html lang>` side effect lives.** Provider or notes
   host? Bias: provider (one place, every consumer benefits).
2. **`SettingsPage` validation feedback for invalid timezone.**
   `Intl.supportedValuesOf("timeZone")` may lag on older Safari.
   Bias: platform default + inline hint.
3. **Reserve `GET /v1/me/preferences/stream` returning 501?** Bias:
   yes, low cost, locks the URL.
4. **Dev catalog watcher placement.** In the existing Vite plugin
   gated by `import.meta.env.DEV`, or separate
   `@nube/starter-ext-vite` package? Bias: same package until a
   second consumer asks.

## Where the canonical design lives

- [`examples/notes/user-pref.md`](../../../examples/notes/user-pref.md)
  — full stage breakdown, decisions D-NP.1 through D-NP.10,
  telemetry table, versioning + deprecation policy, documentation
  deliverables, hard rules inherited from R1–R13.
- [`DOCS/user/scope/SCOPE.md`](../../../DOCS/user/scope/SCOPE.md)
  R9 — extensions consume prefs+i18n through named singletons; one
  source of truth.
- [`DOCS/extensions/scope/SCOPE.md`](../../../DOCS/extensions/scope/SCOPE.md)
  R11 — Module Federation; shared singletons negotiated by host;
  canonical singleton table.

The deep design lives in those three files. **Read them every
stage.** This `SCOPE.md` is the brief that points at them.
