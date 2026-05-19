# User SCOPE — TODO

Tracks open work for the preferences + i18n rollout described in
[SCOPE.md](./SCOPE.md). Phase 0 is done; Phases 1–5 remain. Each
phase below names the crates / surfaces touched and the load-bearing
rules from SCOPE §"Hard rules" that constrain the work.

Phase verification reports land alongside SCOPE.md
(`PHASE<N>-VERIFY.md`). Phase 0's report is
[PHASE0-VERIFY.md](./PHASE0-VERIFY.md).

## Done

### Phase 0 — `starter-spi` wire surface

Status: **shipped**. Job:
[`.codeless/jobs/starter-prefs-spi`](../../../.codeless/jobs/starter-prefs-spi/SCOPE.md).
Commits `984a262` → `61921d4` (stages 1–7).

- [x] `starter-spi::units` — `Quantity`, `Unit`, `QuantityDef`,
      `UnitRegistry`, `StaticRegistry`, `normalize_for_storage`
      (closed enums per R4; all conversions via `uom`).
- [x] `starter-spi::preferences` — `ResolvedPreferences` (no `Option`,
      no `"auto"` per R3), `PreferencesPatch`, and the six display
      enums (`UnitSystem`, `Theme`, `DateFormat`, `TimeFormat`,
      `WeekStart`, `NumberFormat`). All derive `utoipa::ToSchema`
      per workspace R7.
- [x] `starter-spi::i18n` — `LanguageTag`, `MessageKey`,
      `Diagnostic` (with `BTreeMap` params per D-U0.4),
      `DiagnosticParam` (incl. `Timestamp(i64)` epoch-ms per R1).
- [x] New deps on `starter-spi`: `uom = { features = ["si"] }` and
      `icu_locale_core`. No `axum`/`tower`/`hyper`/`chrono` (jiff)/
      `time`/`iso_currency` bleed-in.
- [x] Baseline at
      [`starter-spi-deps.baseline.txt`](./starter-spi-deps.baseline.txt)
      — CI gate for every starter-spi-touching PR.
- [x] Workspace gates green: `cargo build --workspace --all-features`,
      `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
      `cargo fmt --all --check`.
- [x] Headless-appliance smoke holds — a binary linking only
      `starter-auth-token` + `starter-secrets-file` still builds and
      pulls no prefs/i18n machinery.

## Open follow-ups from Phase 0

### F-0.1 — `starter-flow-spi` baseline drift

Severity: **must fix before Phase 1 lands** (Phase 1 will further
mutate the `starter-spi` dep tree and the drift will compound).

`DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` (landed by the
merged `starter-flow-scaffold` sibling) is no longer byte-for-byte
identical to `cargo tree -p starter-flow-spi --edges normal`.
[PHASE0-VERIFY.md](./PHASE0-VERIFY.md) §"starter-flow-spi baseline —
REGRESSION FLAG" enumerates the diff. Two reconciliation paths:

- (a) **Accept the drift.** Re-capture the flow-spi baseline, document
      `uom` + `icu_locale_core` as expected Phase-0 transitive drift,
      and add a worktree-path-stripping step to baseline capture so
      future diffs are stable across worktrees.
- (b) **Restore byte-for-byte.** Move `uom` and `icu_locale_core`
      behind `starter-spi` cargo features (`units`, `i18n`) that
      default to **off**. Phase 1 (`starter-prefs`) and Phase 3
      (`starter-i18n`) opt in explicitly. Preserves the "headless
      appliance pulls nothing it didn't ask for" posture more
      strongly.

Recommendation in PHASE0-VERIFY.md: **(b)**. Decision still pending.

### F-0.2 — Baseline capture is worktree-path-sensitive

Both `starter-spi-deps.baseline.txt` and
`starter-flow-spi-deps.baseline.txt` embed the worktree path on the
top-level crate lines (e.g. `/home/user/.codeless/worktrees/job-…`).
Add a path-stripping post-process to the baseline capture script so
the file is portable across worktrees and CI nodes.

## Phase 1 — `starter-prefs` crate

Status: **not started**. New optional crate per SCOPE §"Crate
layout". Load-bearing rules: R1, R3, R4, R7 (workspace).

- [ ] Crate scaffold: `crates/starter-prefs/` with
      `default-features = []` (workspace R5).
- [ ] Migrations under namespaced source `"starter_prefs"` (workspace
      R4): `0001_starter_prefs.sql` covering `starter_prefs_org` and
      `starter_prefs_user` (SCOPE §"Preferences model"). `BIGINT` on
      Postgres for `updated_at` (R1: avoid 2038 32-bit rollover).
- [ ] `PrefsStore` trait + `sqlite` / `postgres` feature-gated impls
      reusing the typed building blocks from `starter-store-*`.
- [ ] `resolve(user_row, org_row) -> ResolvedPreferences` —
      three-layer resolver. Per-field `user ?? org ?? default`; no
      cross-column overlay. Returns the fully-populated DTO from
      `starter-spi::preferences` (R3).
- [ ] `"auto"` derivation: per-unit fields via `unit_system` table
      (SCOPE §R3), `currency: "auto"` via `iso_currency` locale →
      currency table, `date_format` / `time_format` / `week_start` /
      `number_format` via ICU defaults. Hardcoded system default
      only when ICU has no opinion.
- [ ] `iso_currency` dep lands here per D-U0.3 (not in `starter-spi`).
- [ ] REST routes behind `routes` feature: `GET`/`PATCH
      /v1/me/preferences`, `GET`/`PATCH /v1/orgs/{id}/preferences`
      (`require_role(Admin)`), `GET /v1/units` (ETag +
      `X-Platform-Version`). All derive `ToSchema` (workspace R7).
- [ ] `starter-client-rs` methods: `get_my_preferences`,
      `patch_my_preferences`, `get_org_preferences`,
      `patch_org_preferences`, `get_units`.
- [ ] `starter-cli prefs` subcommand (get / set / org get / org set).
- [ ] Single-tenant fallback: reserved sentinel
      `workspace_id = "@starter/default"`; enforce `@`-prefix
      reservation in `starter-auth-users` org-creation path.
- [ ] Multi-org via `Principal`'s active org + optional `?org=`
      query param on `/v1/me/preferences`.
- [ ] Smoke tests from SCOPE §"Smoke tests": "Resolver layer
      precedence", "`auto` derivation" (incl. the BBQ case),
      "Headless appliance keeps working" still green.
- [ ] `starter-spi-deps.baseline.txt` re-captured (Phase 1 may shift
      transitives; bake the new baseline into the same commit).

## Phase 2 — `Accept-Units` middleware + per-series response shape

Status: **not started**. Lives in `starter-server`. Load-bearing
rules: R6 (single conversion layer per surface), R7 (custom header),
R8 (per-series metadata).

- [ ] `accept_units_layer(registry, prefs_resolver)` tower layer:
      reads `Accept-Units` (`preferred` default, `canonical` for
      MCP), resolves prefs once per request, inserts `UnitsCtx` into
      request extensions, sets `Vary: Accept-Units`. **Does not
      mutate response bodies** (R6); conversion is opt-in per
      handler via `UnitsCtx::convert(quantity, value, source_unit)`.
- [ ] Per-series response shape per R8: `{ slot, quantity, unit,
      points: [[ts_ms, value], …] }`. Single-value reads use the
      inline `{ value, unit, quantity }` form.
- [ ] No-op when `starter-prefs` is not compiled in.
- [ ] CDN-cache-key operator note: docs warn that `Vary` is advisory
      and CloudFront / Fastly / Cloudflare need explicit edge config
      to key on `Accept-Units` / `Accept-Language` (SCOPE §R7).
- [ ] Audit pass: grep starter crate logs during integration tests
      for `"°F"`, `"psi"`, `"mph"`, `"lb"` — must be zero matches
      (SCOPE §"Canonical-only logs" smoke).
- [ ] Smoke tests: "Australian operator", "MCP raw mode" green.

## Phase 3 — `starter-i18n` crate

Status: **not started**. New optional crate per SCOPE §"Crate
layout". Load-bearing rules: R5 (client-side default), R7.

- [ ] Crate scaffold: `crates/starter-i18n/` with
      `default-features = []`.
- [ ] `LanguageTag` already lives in `starter-spi::i18n` (Phase 0).
      Add `Accept-Language` parser + fallback chain (requested →
      family → `en`).
- [ ] JSON catalog format + loader (`catalog.rs`); compiled-in
      starter-owned catalogs (`platform.rs`).
- [ ] `MessageBundle` with fallback chain. Missing keys fall through
      to the source string (never error). Every fallback emits a
      `tracing::debug!`; opt-in `X-I18n-Fallback` response header
      (off by default, enable per route or globally via
      `accept_language_layer().with_fallback_header(true)`).
- [ ] `accept_language_layer(bundle)` tower layer: picks language,
      sets `Content-Language` + `Vary: Accept-Language`, inserts
      `LocaleCtx`.
- [ ] Routes behind `routes` feature: `GET
      /v1/i18n/catalogs/{language}` (ETag), `GET
      /v1/i18n/catalogs/{language}-{fingerprint}.json` (immutable
      content-hash URL), `GET /v1/i18n/manifest`.
- [ ] Seed `catalogs/starter/en.json` + `catalogs/starter/es.json`
      covering starter's own UI strings (auth, errors, settings
      chrome).
- [ ] Smoke test: "Add a language" — dropping `fr.json` requires no
      backend deploy for clients to use it.

## Phase 4 — `@nube/starter-ui-core` TypeScript surface

Status: **not started**. Workspace R6 split: TS client zero React;
UI-kit zero I/O; UI-core owns the brain.

- [ ] `@nube/starter-client-ts`: regenerate from `openapi.json` —
      `ResolvedPreferencesSchema`, `PreferencesPatchSchema`,
      `UnitRegistryDtoSchema`, plus methods (`getMyPreferences`,
      `patchMyPreferences`, `getOrgPreferences`,
      `patchOrgPreferences`, `getUnits`, `getI18nCatalog`).
- [ ] `@nube/starter-ui-core/preferences/`:
      `PreferencesProvider.tsx` (React Query + ETag caching),
      `usePreferences.ts`, `useUpdatePreferences.ts` (optimistic),
      `formatters.ts` (pure functions, no React: `formatDate`,
      `formatTime`, `formatDateTime`, `formatRelativeTime`,
      `formatNumber`, `formatUnit`, `formatCurrency`). Query keys
      namespaced `['starter', 'prefs', …]` per workspace R6.
- [ ] `@nube/starter-ui-core/i18n/`: `IntlProvider.tsx` (wraps
      `react-intl`, reads `prefs.language`), `useTranslate.ts`,
      `registerExtensionMessages.ts`.
- [ ] `@nube/starter-ui-core/i18n/catalogs/` seed `en.json` +
      `es.json` (mirrors the backend seed).
- [ ] `@nube/starter-ui-kit`: `<ThemeProvider>` reads `theme` from a
      context provided by ui-core (no new I/O in ui-kit). Tailwind
      preset adds relative-time tokens.
- [ ] Settings page wired into `starter-auth-users`' account page.
- [ ] Theme cold-start: persist to `localStorage` so SPA paint is
      correct before the network round-trip. SSR cookie path is
      explicitly out of v1.

## Phase 5 — Diagnostics rewriter

Status: **not started**. Opt-in feature on `starter-i18n`. Closes
the one documented exception to R5 (client-side translation).

- [ ] Scope-limited rewriter: runs only when the handler inserts a
      `DiagnosticBody` extension on the response; touches only the
      declared envelope shape (`{ diagnostic: { code, params } }`)
      at documented top-level paths. **Does not** walk arbitrary
      JSON bodies. **Does not** rewrite streaming responses (SSE,
      chunked) — those emit codes + params and the consumer
      translates per event.
- [ ] Use for server-originated long-running async jobs, audit-trail
      messages, scheduled exports / emails.
- [ ] Server-originated emails / exports re-resolve prefs from the
      DB at send time (never a cached JWT claim) per SCOPE §R6.
