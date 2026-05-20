# Scope — user preferences in `examples/notes` extension + MF host

> Source-of-truth ceilings:
> [`DOCS/user/scope/SCOPE.md`](../../DOCS/user/scope/SCOPE.md) (R1–R8 —
> the preferences/i18n rules already shipped in PRs #8/#10) and
> [`DOCS/extensions/scope/SCOPE.md`](../../DOCS/extensions/scope/SCOPE.md)
> (R1–R13 — the extension substrate). This file is a per-example scope
> for wiring the existing prefs+i18n surface into the `notes` host and
> through one Module-Federation extension. When this file disagrees
> with either source, those win.
>
> **Framework posture.** This example is also the reference
> implementation that every starter-based product copies when it wires
> its own extensions. The contracts named here (singleton ids, SDK
> hook surface, catalog packaging, telemetry events, semver promise)
> are therefore **frozen on merge** — not "an example we'll generalise
> later." Stage 7 exists for exactly this reason: the production-grade
> rules ship in the same PR as the demo so consumer teams do not have
> to re-discover them.

## Goal

Take the prefs + i18n + Module-Federation pieces that **already exist
on master** and prove they work end-to-end inside
[`examples/notes`](.):

1. The notes app, opened by an Australian operator
   (`Australia/Brisbane`, `en-AU`, `unit_system: metric`,
   `temperature_unit: F` for the BBQ override), renders dates, times,
   numbers, and any unit values in AU conventions out of the box.
2. The hello extension that ships with the notes app
   ([`extensions/com.nube.hello`](extensions/com.nube.hello/block.yaml))
   uses the **same** resolved prefs and the **same** translated
   strings as the host — no second copy of the data, no second
   round-trip.
3. Switching language to Spanish in the host's Settings page flips
   both the host chrome and the federated `HelloPanel`'s strings in
   one render, without a page reload.
4. The same flip propagates to a second open browser tab within one
   animation frame — production teams will hit multi-tab immediately
   and the demo must not silently get this wrong.

The example is the consumer-facing proof that R6 ("convert at exactly
one layer per surface") and R11 ("MF; shared singletons negotiated
host-side") hold across the extension boundary — **and** the
load-bearing reference for every starter-based product that follows.

## Why this exists

The prefs/i18n SCOPE shipped Phase 4 (`@nube/starter-ui-core`
`PreferencesProvider`, `formatters`, `IntlProvider`, `SettingsPage`)
but never said how a Module-Federated extension consumes them. The
extensions SCOPE shipped R11 (singletons negotiated host-side) but
never named the prefs/i18n contexts as singletons. Today, an
extension author has three bad options:

- Refetch `GET /v1/me/preferences` themselves — duplicates network +
  state.
- Re-create an `IntlProvider` inside the panel — duplicates the
  catalog fetch and, worse, drifts when the host's language flips.
- Hard-code `Intl.DateTimeFormat()` with no locale — silently ignores
  the user's settings.

This scope picks **option zero**: the host's `PreferencesProvider` and
`IntlProvider` are exposed to extensions via the
`ExtensionHostManager`'s singleton channel, with a tiny
`@nube/starter-ext-sdk-ts` hook surface (`useHostPrefs`,
`useHostTranslate`, `useHostFormatters`) that mirrors the existing
`useHostClient`. One source of truth, one fetch, one re-render.

## In scope

### Stage 1 — notes host wires prefs + i18n

- Add `PreferencesProvider` and `IntlProvider` from
  `@nube/starter-ui-core` around the notes app in
  [`frontend/src/app.tsx`](frontend/src/app.tsx). Both providers wrap
  *inside* the existing client provider so they can read the bearer
  token.
- Add a Settings tab (or a top-bar gear icon) that renders
  `<SettingsPage />` from `@nube/starter-ui-core`. No new UI
  primitives — `SettingsPage` already covers every column in the
  prefs model.
- Set the document root `lang` attribute from the resolved language
  (`document.documentElement.lang = resolved.language`). Screen
  readers and browser UA (hyphenation, spell-check, font fallback)
  depend on it; teams that forget this ship a broken a11y story and
  do not notice. The provider owns the side effect so every consumer
  gets it for free.
- Define the **loading contract**: while prefs are unresolved,
  `PreferencesProvider` renders its `fallback` prop (default: a
  skeleton; the notes host passes its existing top-bar skeleton).
  No formatter ever runs against `undefined` prefs — the hooks in
  Stage 3 suspend instead of returning partial values.
- Sqlite migrations for `starter-prefs` already register through
  `starter-store-sqlite`'s namespaced runner; the notes server enables
  the `starter-prefs` and `starter-i18n` cargo features in
  [`Cargo.toml`](Cargo.toml) so the routes mount and the catalogs
  serve.
- Smoke test (Vitest, headless): mount the app with a stub client,
  resolve prefs to `en-AU` + metric + `temperature_unit: F`, render a
  fixture timestamp + a fixture temperature, assert `22/04/2026` and
  `72.4 °F` appear, **and** assert `document.documentElement.lang ===
  "en-AU"`. One test file under
  [`frontend/src/`](frontend/src) named `prefs-host.test.tsx`.

### Stage 2 — singleton channel for prefs + i18n contexts

- Extend `ExtensionHostManager`'s singleton table with two new
  singletons:
  - `"@nube/starter-ui-core/preferences"` — exposes the
    `PreferencesContext` value (resolved prefs + the
    `setPreferences(patch)` callback) so an extension can `useContext`
    against the **host's** instance, not a duplicate.
  - `"@nube/starter-ui-core/i18n"` — exposes the host's
    `react-intl` `IntlShape` so `useIntl()` inside an extension
    panel resolves against the host's catalog and language.
- The handshake matches R11: extension declares
  `singletons: { "@nube/starter-ui-core/preferences": { version: "<major>" } }`
  in its `init` factory; host refuses to register on major mismatch
  (`Failed` lifecycle, reason `singleton-mismatch: <pkg>@<major>`,
  other extensions unaffected). The host emits one
  `extension.singleton_mismatch` telemetry event per failure (see
  Telemetry section) so a production deployment surfaces the
  mismatch in dashboards instead of in console logs the operator
  never reads.
- **Minor-version handling.** A minor mismatch (host on `1.3`,
  extension declared `1.1`) loads with a `warn`-level telemetry
  event (`extension.singleton_minor_drift`); host stays compatible
  but the platform team has a tripwire when adoption lags. Patch
  drift is silent.
- `extension-host.ts` adds the two singletons to the
  `createExtensionHost` call alongside the existing React/ReactDOM
  pair. No new manifest fields; the singleton list is the contract.

### Stage 3 — `@nube/starter-ext-sdk-ts` hooks

The hooks live in the SDK (not the host runtime) so extension authors
import from one stable surface. All three are thin wrappers — no
state, no fetch, no caching of their own.

- `useHostPrefs(): ResolvedPreferences` — returns the resolved
  preferences from the host's `PreferencesContext`. Throws (with a
  documented message) if called outside an extension mounted by the
  notes host so the wrong-context mistake is loud, not silent.
- `useHostTranslate(): (key: MessageKey, params?: …) => string` —
  returns a typed translator backed by the host's `IntlShape`. The
  `MessageKey` type is **generated** from the union of platform
  catalog keys + the calling extension's catalog keys (Stage 5
  build step), so a typo on a key is a TypeScript error, not a
  silent "translation missing" string at runtime.
- `useHostFormatters(): { formatDate, formatTime, formatNumber,
  formatCurrency, formatQuantity }` — pure functions already exported
  from `@nube/starter-ui-core/preferences/formatters.ts`, bound to
  the resolved prefs from `useHostPrefs` so extension authors don't
  have to thread `prefs` through every call site.
- Ship a `<MockHostProvider prefs={…} language={…} catalogs={…}>`
  test helper in `@nube/starter-ext-sdk-ts/testing`. Extension
  authors mount their panel under it in unit tests without booting
  the notes host. Without this, every team will roll their own
  mock and drift will leak into production panels.
- Unit tests with `@testing-library/react` mount a panel under a fake
  host providing the same singleton shape and assert each hook
  returns the expected value.

### Stage 4 — `com.nube.hello` consumes the singletons

- `extensions/com.nube.hello/ui/remoteEntry.js` is rewritten (still
  hand-written, still ESM, still SCOPE R7 — static metadata only) so
  the `init` factory:
  - Declares the two new singletons in `singletons:`.
  - Reads them off the `handle` and threads them into the panel.
  - Renders one **localised + formatted** line per surface:
    - `useHostTranslate()("hello.greeting", { name })` — the
      greeting itself comes from the catalog, so flipping language
      flips the panel.
    - `useHostTranslate()("hello.unread", { count })` — an ICU
      plural (`{count, plural, one {# unread note} other {# unread
      notes}}`) proves the catalog format is plural-aware, not
      string-replacement. Production teams will need plurals on
      day one; demoing them here is cheaper than discovering the
      gap in five product teams.
    - `useHostFormatters().formatDate(Date.now())` — proves the
      AU-formatted date matches the host chrome.
    - `useHostFormatters().formatQuantity(22.44, "temperature",
       "celsius")` — proves the BBQ override (`temperature_unit: F`)
       flips the panel's display from `22.4 °C` to `72.4 °F` without
       the panel knowing anything about units beyond the canonical
       value.
- Add `hello.greeting` to
  [`crates/starter-i18n/catalogs/starter/{en,es}.json`](../../crates/starter-i18n)
  — **wait**, no. R7 of the prefs SCOPE says block authors register
  catalogs via `registerExtensionMessages`. The extension carries its
  own translations in
  `extensions/com.nube.hello/i18n/{en,es}.json`, loaded by the host's
  `IntlProvider` at registration time. The platform catalog is
  untouched.

### Stage 5 — `block.yaml` declares the new singleton needs

- Add `requires: ["@nube/starter-ui-core/preferences",
  "@nube/starter-ui-core/i18n"]` to
  [`com.nube.hello/block.yaml`](extensions/com.nube.hello/block.yaml).
  Capability-style declaration matches extensions R6 ("capabilities
  declared, granted, enforced") — the host refuses to load the
  extension if the singletons are not present in its registry.
- Add a `contributes.i18n.catalogs: { en: "i18n/en.json",
  es: "i18n/es.json" }` block under `contributes`. The notes host
  serves the extension's catalog files alongside `ui/remoteEntry.js`
  off `GET /extensions/<id>/i18n/<lang>.json`; the SDK's
  `registerExtensionMessages` helper merges them into the host's
  bundle namespaced under the extension id (`com.nube.hello.*` keys)
  so they cannot collide with platform strings. The merge **lazy-
  loads per active language only** — the host fetches `es.json`
  the first time the user flips to Spanish, never all locales
  up-front. Five extensions × ten locales × 50 KB each would
  otherwise be a 2.5 MB upfront cost no team would accept.
- Add a CI gate: `pnpm -w run check:i18n` walks every
  `i18n/<lang>.json` under the workspace and asserts (a) every
  non-`en` catalog has the same key set as `en`, (b) every key
  resolves to a non-empty string, (c) ICU placeholders match across
  languages. Missing keys fail the build. Production teams that
  inherit this framework cannot ship a partially-translated locale
  by accident.
- The SDK emits an `extension.catalog_key_collision` telemetry event
  if an extension catalog declares a fully-qualified key inside
  another extension's namespace (e.g. `com.nube.other.greeting`).
  The key is dropped from the merge; the warning surfaces in the
  registry detail page.

### Stage 6 — Playwright e2e

One spec under
[`frontend/e2e/`](frontend/e2e):

- Start the notes server, open the app as the `au-bbq-operator`
  fixture user (locale `en-AU`, unit_system `metric`,
  temperature_unit `F`, language `en`).
- Assert the sidebar's `HelloPanel` shows the English greeting and a
  `°F` temperature.
- Assert `document.documentElement.lang === "en-AU"`.
- Open Settings, switch language to `es`, save.
- Assert the panel re-renders with the Spanish greeting **without a
  reload** and the temperature is still `°F`. Assert
  `document.documentElement.lang === "es"` (or `es-AU` if the
  locale resolver preserved the region — pin whichever the
  resolver actually emits; the test is the spec).
- Open a second tab pointed at the same notes URL. Without
  interacting, assert the panel and chrome in tab two are already
  in Spanish — proves the BroadcastChannel propagation from D-NP.9.
- Switch `temperature_unit` to `C`, save. Assert the panel now shows
  `°C`. Host chrome and panel agree. Assert tab two follows.

### Stage 7 — production hardening cross-cuts

The cross-cuts that do not belong to one stage but must ship in this
PR for the framework to be production-grade. Each is small; together
they are what separates "demo that works" from "framework five teams
adopt without filing the same five bugs."

- **Locale fallback chain.** `IntlProvider` resolves a requested
  language by truncating BCP-47 tags left-to-right until a catalog
  hits, falling through to `en` as the floor (D-NP.6). The resolver
  emits one `i18n.locale_fallback` telemetry event the first time
  it falls back for a given session so the platform team can see
  which locales have catalog gaps.
- **Missing-key telemetry.** `useHostTranslate` returns the key
  itself when no catalog hit is found (react-intl default), but
  emits an `i18n.message_missing` telemetry event with the key,
  language, and extension id. In dev, the event also `console.warn`s
  with a stack so the author sees it. In prod, only the counter
  fires.
- **Multi-tab propagation.** `PreferencesProvider` writes every
  successful `setPreferences` to a `BroadcastChannel("starter-
  prefs")` and listens on the same channel; other tabs apply the
  patch optimistically and re-validate against the server. Closing
  the gap server-side (a long-poll or SSE on `/v1/me/preferences`)
  is a follow-up explicitly out of scope here; the BroadcastChannel
  covers the same-browser case that every demo will run into first.
- **Dev catalog watcher.** In `vite dev`, the i18n bundle plugin
  watches `extensions/**/i18n/*.json` and hot-replaces the merged
  bundle on change. Disabled in prod (`if (import.meta.env.DEV)`).
  Without this, every catalog edit is a server restart and
  extension authors will hard-code strings just to iterate faster.
- **Perf budget.** A single `setPreferences` call must not trigger
  more than one render per consumer of the context. The
  `PreferencesProvider` memoises its context value on the resolved
  prefs object identity; a Vitest renders three sibling consumers
  and asserts each renders exactly twice across a language flip
  (initial + post-flip). Catches the easy regression where someone
  inlines an object literal into the provider's `value=` prop.
- **A11y.** When the language flips, the provider toggles an
  `aria-live="polite"` region with the new language's display name
  (`"Idioma cambiado a Español"`). Screen-reader users get told
  the page changed language; without this, the flip is silent.

## Out of scope

- A new framework. No `@nube/starter-ui-prefs-react` package; the
  hooks live in the existing `@nube/starter-ext-sdk-ts` package next
  to `useHostClient`.
- Per-extension preference overrides. Extensions read; they do not
  store their own prefs. A "block-defined preference" surface is a
  v2 conversation explicitly deferred in the prefs SCOPE.
- The Rust `Ctx::prefs()` / `Ctx::locale()` accessor for backend
  extension code. Real and useful, but server-side and unrelated to
  the MF wiring — tracked separately in the extensions-addendum
  scope. **Cross-link:** when that scope lands, the SDK gains a
  matching `Ctx::translate()` so backend logs and server-emitted
  diagnostics flow through the same catalog the UI uses. Flagged
  here only so the framework consumer knows the seam exists.
- Server-side rendering of extension panels. The federation runtime
  is client-only; SSR is out of v1 in the prefs SCOPE and equally
  out here. Teams that adopt Next.js wrapping the notes shell will
  need to render the chrome server-side and the federated panels
  client-only; documented in the consumer guide, not solved here.
- Hot-reload of catalogs in **prod**. Dev watcher ships (Stage 7);
  prod still requires a redeploy.
- WASM and process extensions. This scope wires a builtin/MF UI
  extension only. The trait does not change (extensions R13) so the
  same singleton handshake works for any future UI extension running
  in WASM or process, but the demo extension stays builtin.
- Bidirectional (RTL) language support. The provider sets `dir`
  alongside `lang` from a known RTL list (`ar`, `he`, `fa`, `ur`)
  so the chrome renders correctly when those catalogs are added,
  but no RTL catalog ships in this PR and no RTL e2e runs.
- Server-side translation of error responses. The server still emits
  stable message keys + params per the prefs SCOPE R5 documented
  exception; the client renders. Production teams that need
  translated emails or PDFs use the same catalog from a server-side
  bundle — out of scope here, explicitly tracked as a v2.

## Hard rules (inherited)

- **R6 (prefs)** — Conversion at exactly one layer per surface. The
  extension panel calls `formatQuantity` and gets the formatted
  string; it never touches `uom`, never sees a non-canonical input,
  never converts.
- **R8 (prefs)** — Per-series unit metadata. Any list of values the
  panel renders gets one `{quantity, unit}` declared at the series
  level, not per row. The hello panel only renders one value, so
  the rule is structural; the smoke test still asserts the shape.
- **R7 (extensions)** — Static metadata, never runtime-templated. The
  `hello.greeting` catalog entry uses ICU placeholders (`{name}`),
  not server interpolation. The catalog is a file, not a template
  the server fills in.
- **R11 (extensions)** — Module Federation; shared singletons
  negotiated host-side. The new prefs + i18n singletons live in the
  same registry as React and ReactDOM, with the same version-major
  refusal-to-load semantics.
- **R5 (prefs)** — Translation is client-side. The host's
  `IntlProvider` looks up the catalog; the server emits no
  translated strings into the extension panel's data path.

## Decisions (lock before code)

### D-NP.1 — Singleton ids are the package + subpath

`@nube/starter-ui-core/preferences` and
`@nube/starter-ui-core/i18n`, **not** ad-hoc strings like
`"prefs"`. Matches the React / ReactDOM convention already in
`extension-host.ts` (the singleton key is what the extension would
`import`). Revisit if a consumer asks for a non-`@nube` extension
that ships its own prefs surface.

### D-NP.2 — Hooks live in `@nube/starter-ext-sdk-ts`, not `ui-core`

UI-core owns the contexts and the providers; the SDK exposes the
*hooks* that read those contexts via the singleton handle. Drawing
the line here keeps `ui-core` free of any extension-host concept and
keeps the SDK as the one import path an extension author types.
Revisit if a non-extension consumer wants the same hooks (then
promote them to `ui-core`).

### D-NP.3 — Extension catalogs are namespaced by extension id

A key written in `com.nube.hello/i18n/en.json` as `greeting`
registers into the host's bundle as `com.nube.hello.greeting`.
`useHostTranslate` accepts both: the SDK auto-prefixes with the
current extension's id when the key has no dot, and passes through
fully-qualified keys verbatim. Avoids cross-extension key collisions
and avoids extensions accidentally overwriting platform strings.

### D-NP.4 — No new manifest schema version

The new `contributes.i18n.catalogs` and `requires` entries fit in
manifest `v: 1` because the manifest already deny-unknown-fields's
on the **fields it knows about** but `contributes.*` keys are open
per the extensions SCOPE (R13 — adding a `contributes.<x>` block is
how new transports land). Adding `i18n` is the same shape change as
adding `workers` was.

### D-NP.5 — The notes app stays the only consumer in this scope

`examples/gh-report` and `examples/minimal` are not updated. The
sole goal is one end-to-end demo. If a second consumer needs the
same wiring, the lift-out path is obvious (the `extension-host.ts`
becomes a `createPrefsAwareHost()` helper exported from
`@nube/starter-ext-ui`); that lift-out is a follow-up, not a
prerequisite.

### D-NP.6 — Locale fallback chain is left-truncating BCP-47, floor `en`

`es-MX` → tries `es-MX`, then `es`, then `en`. The floor is `en`,
not the first available catalog, because "first available" is
unstable as catalogs are added. Languages without an `en` floor
would block the whole render — unacceptable for a framework. The
resolver is pure; the same function runs in the smoke test fixture.

### D-NP.7 — Catalog format is flat ICU MessageFormat

Keys are flat strings (`hello.greeting`, not nested objects).
Values are ICU MessageFormat (`{count, plural, …}`,
`{name, select, …}`). Flat keys keep the codegen for `MessageKey`
trivial and prevent the "should `hello.greeting.formal` shadow
`hello.greeting`?" debate. ICU is what `react-intl` already parses;
no additional runtime.

### D-NP.8 — Catalogs lazy-load per active language

The host bundles only the platform `en` catalog into the initial
JS bundle. Every other language (platform + extension) loads on
first need over `fetch`. Initial paint is identical whether one
or fifty extensions ship catalogs. The fetched JSON is cached in
the browser HTTP cache; immutable URLs (content-hashed) make this
free.

### D-NP.9 — Multi-tab prefs propagation is BroadcastChannel-only

No server-side push for this scope. `BroadcastChannel("starter-
prefs")` covers same-browser multi-tab, which is the 95th-percentile
case (an operator with two tabs of the same product). Cross-device
propagation is a v2 conversation tied to the prefs API's eventual
SSE/long-poll surface; flagged in Out of scope.

### D-NP.10 — Semver promise for the singleton contract

`@nube/starter-ui-core/preferences@<major>` and
`…/i18n@<major>` follow standard semver:

- Major bump — `ResolvedPreferences` shape changes, hook signatures
  change, `IntlShape` consumers must re-test. Host refuses to load
  extensions declared against an older major.
- Minor bump — additive: new prefs field, new hook, new formatter.
  Extensions on older minors keep loading; the host emits the
  `extension.singleton_minor_drift` event from Stage 2.
- Patch — internal: bug fixes, perf. Silent.

Deprecations land as a minor bump that emits a console warning
plus a telemetry event for two minor versions before removal in
the next major. The exact policy lives in
[`DOCS/user/scope/SCOPE.md`](../../DOCS/user/scope/SCOPE.md) when
that file is amended — until then, this scope is the source.

## Telemetry + observability

Every event listed below is emitted through the existing
`starter-observability` event bus on the host (`emit_extension_event`
in Rust, mirrored client-side via the host's existing telemetry
sink). Names are stable — production dashboards will key off them.

| Event | Fired when | Severity |
|---|---|---|
| `extension.singleton_mismatch` | Major version mismatch refused load (Stage 2) | error |
| `extension.singleton_minor_drift` | Minor mismatch loaded with warning (Stage 2) | warn |
| `extension.catalog_key_collision` | Extension wrote into another's namespace (Stage 5) | warn |
| `i18n.locale_fallback` | First fallback per session per locale (Stage 7) | info |
| `i18n.message_missing` | Translation key not found (Stage 7) | warn |
| `prefs.broadcast_dropped` | BroadcastChannel post failed (Stage 7) | warn |

No PII flows through these events. The catalog key is part of the
public manifest surface; the language tag is BCP-47.

## Versioning + deprecation

- The singleton contract follows D-NP.10. Hosts and extensions
  negotiate by major; minor drift is a tripwire, not a failure.
- The SDK package (`@nube/starter-ext-sdk-ts`) versions in lockstep
  with the singleton major. Bumping the SDK major is the only way
  to break the hook signatures, and that bump propagates through
  every starter-based product on a known cadence.
- The `block.yaml` manifest is `v: 1` for this scope and the
  foreseeable future (D-NP.4). New `contributes.*` keys are
  additive; renaming or removing one requires a manifest `v: 2`
  which is **not** something this PR touches.
- Catalog files (`i18n/<lang>.json`) have no version field; the key
  set itself is the contract. Adding a key is a minor change to the
  shipping extension; removing one is a breaking change to any
  consumer that may have hand-written that key into another
  catalog. The Stage 5 CI gate enforces parity.

## Documentation deliverables

Three short docs ship in the same PR as the code; the framework's
adoption story is the docs as much as the code:

- `DOCS/extensions/guides/i18n.md` — "Localising your extension."
  Walks through `block.yaml` declaration, catalog format, ICU
  plurals, the SDK hooks, and the `MockHostProvider` test pattern.
  Reference, not tutorial — ten minutes to read top-to-bottom.
- `DOCS/user/guides/prefs-in-extensions.md` — Operator-facing.
  Explains why "set language once" affects panels written by other
  teams.
- `examples/notes/extensions/com.nube.hello/README.md` — Updates
  the hello extension's own README so a team copying it as a
  starting point sees the i18n hooks in context, not just the
  manifest.

Without these, every new starter-based product will re-derive the
wiring from the diff and drift will start on day one.

## Constraints

- TypeScript strict, ESM only, no new top-level dependencies in
  `examples/notes/frontend/package.json` beyond what `ui-core` /
  `ext-sdk-ts` / `ext-ui` already pull (`react-intl` is already
  transitively in via `ui-core`).
- `pnpm -w build` and `pnpm -w test` stay green.
- `pnpm -w run check:i18n` (new, Stage 5) is wired into the
  workspace `check` script and runs in CI.
- Server side: `cargo check -p starter-notes` and `cargo test -p
  starter-notes` stay green; no new Rust deps in `notes/Cargo.toml`
  beyond `starter-prefs` + `starter-i18n` (both already in the
  workspace).
- Playwright spec runs against the existing `playwright.config.ts` —
  no new browser config.
- Manifest stays `v: 1`. No schema bump.
- Initial JS payload for `examples/notes/frontend` grows by **no
  more than 8 KB gzipped** over master. Measured by the existing
  `pnpm -w run analyze:bundle` script; the threshold lands in the
  CI gate.
- Render budget: a single `setPreferences` call causes at most one
  re-render per consumer (Stage 7 test).
- A11y: axe-core run in the Playwright spec reports zero new
  violations against master.

## Smoke tests (merge gates)

In addition to the platform smoke tests already living in
`crates/smoke-tests/`:

- **`prefs-host.test.tsx`** — host renders AU-formatted date + BBQ
  temperature against a stub client, asserts `<html lang>` set
  (Stage 1).
- **`extension-prefs-singleton.test.tsx`** — a fake extension mounted
  under a fake host reads `useHostPrefs` / `useHostTranslate` /
  `useHostFormatters` and gets the host's values (Stage 3).
- **`extension-catalog-merge.test.tsx`** — registering an extension's
  `i18n/en.json` adds namespaced keys to the host's bundle without
  evicting platform keys; collision warning fires when expected
  (Stage 5).
- **`locale-fallback.test.ts`** — pure-function test of the
  D-NP.6 resolver covering `es-MX` → `es`, `pt-BR` → `pt` → `en`,
  unknown → `en` (Stage 7).
- **`prefs-render-budget.test.tsx`** — three sibling consumers
  re-render exactly twice across a language flip (Stage 7).
- **`au-operator.spec.ts`** — Playwright end-to-end of the Stage 6
  flow above (language flip + unit flip without reload, second-tab
  propagation, `<html lang>` flip, axe-core clean).

The six tests are the merge gate. None of them needs a real LLM, a
real OAuth handshake, or a real Postgres — they all run against the
in-process sqlite store and the notes shell's stub client, matching
the rest of `examples/notes/`.

## Rollout

One PR, seven stages, ordered:

1. Stage 1 — host providers + Settings tab + `<html lang>` +
   Stage-1 smoke.
2. Stage 2 — singleton channel additions in
   `@nube/starter-ext-ui` + host wiring in `extension-host.ts` +
   mismatch telemetry.
3. Stage 3 — `@nube/starter-ext-sdk-ts` hooks + `MockHostProvider`
   + `MessageKey` codegen + Stage-3 smoke.
4. Stage 4 — `com.nube.hello` panel rewrite to use the hooks
   (greeting, plural, formatted date, BBQ temperature).
5. Stage 5 — `block.yaml` requires + extension catalog wiring +
   `check:i18n` CI gate + Stage-5 smoke.
6. Stage 6 — Playwright e2e (single + multi-tab + a11y) + final
   docs sweep.
7. Stage 7 — production cross-cuts (fallback chain, missing-key
   telemetry, BroadcastChannel, dev catalog watcher, perf budget
   test, a11y live region) + the three documentation deliverables.

Each stage commits its tests in the same commit as the body
(workspace-wide rule). Stages 1–3 are independently reversible;
Stages 4–7 are coupled (the panel rewrite assumes the hooks, the
cross-cuts assume the wiring).

## Open questions

1. **`SettingsPage` validation feedback for invalid timezone.**
   `Intl.supportedValuesOf("timeZone")` is the source list, but old
   Safari may lag. Bias: render the platform default and surface a
   "timezone changed since your browser was updated" hint inline.
   Resolve in Stage 1 against the live `SettingsPage` props.
2. **Where the `<html lang>` side effect lives.** Either in
   `PreferencesProvider` (one place, every consumer benefits) or in
   the notes host (explicit, but every consumer re-implements).
   Bias: provider, because the framework posture in the preamble
   makes "every consumer re-implements" the wrong default.
   Resolve in Stage 1.
3. **Server long-poll / SSE for cross-device prefs sync.** D-NP.9
   keeps it out of this scope; the question is whether the prefs
   API should ship a placeholder `GET /v1/me/preferences/stream`
   route returning 501 so the URL is reserved. Bias: yes, reserve.
   Resolve before merge, low cost.
4. **Whether the dev catalog watcher belongs in the Vite plugin or
   in a separate `@nube/starter-ext-vite` package.** Bias: same
   package, gated by `import.meta.env.DEV`, until a second consumer
   asks for it stand-alone. Resolve in Stage 7.
