# Scope — starter-prefs-i18n

> Source of truth: [`DOCS/user/scope/SCOPE.md`](../../../DOCS/user/scope/SCOPE.md)
> §"Rollout (proposed phases)" Phases 1–5, plus the upstream
> Hard-rules block (R1–R8). This file is the per-job brief;
> intentionally short. When this file disagrees with the
> source-of-truth SCOPE, that doc wins.

## Goal

Land **Phases 1 through 5 in one branch** plus the two open
Phase 0 follow-ups (F-0.1 and F-0.2 from
[`PHASE0-VERIFY.md`](../../../DOCS/user/scope/PHASE0-VERIFY.md)).
This is the deliberate "one big job" shape — every phase of the
user SCOPE merges together, on `codeless/starter-prefs-i18n`,
because the user has chosen that posture for this work and
codeless's stage + REVIEW machinery is built to make it
tractable.

After this job, a starter-based product compiled with the new
crates and the augmented `@nube/starter-ui-core` gets
multi-locale, multi-unit, multi-timezone behaviour out of the
box: locale, language, timezone, unit system, per-quantity unit
overrides, date/time/number formats, currency, theme — with the
three-layer (user → org → default) resolver, canonical-only
storage, edge-of-API conversion, ICU-driven catalogs, and a
Settings page wired into the existing account surface.

## In scope (per phase)

### Phase 0 closure (stage 1)

- **F-0.1**: feature-gate `uom` + `icu_locale_core` on `starter-spi`
  so the Phase 0 dep landing does not leak into
  `starter-flow-spi`'s tree. Update
  [`DOCS/user/scope/starter-spi-deps.baseline.txt`](../../../DOCS/user/scope/starter-spi-deps.baseline.txt)
  to reflect the feature-gated state.
  [`DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`](../../../DOCS/flow/scope/starter-flow-spi-deps.baseline.txt)
  stays byte-for-byte unchanged once the gating lands.
- **F-0.2**: add a baseline-capture script at
  `DOCS/user/scope/capture-baseline.sh` that strips worktree
  paths so `cargo tree -p starter-spi --edges normal` output is
  reproducible across worktrees. Same script applies to the
  flow-spi baseline in a separate small follow-up — not
  bundled here.
- Lock the Phase 1–5 decisions (D-PI.1 through D-PI.7 below).

### Phase 1 — `starter-prefs` (stages 3–7)

- New crate at `crates/starter-prefs/` with
  `default-features = []`.
- Pure-function three-layer resolver per R3 (no Option, no
  "auto" in `ResolvedPreferences`).
- `PrefsStore` trait + sqlite impl behind a `sqlite` feature.
  Postgres impl is deferred per D-PI.2; the SCOPE Crate-layout
  block lists `starter-store-sqlite` explicitly, and shipping
  Postgres later is the established workspace posture.
- Four REST endpoints behind a `routes` feature:
  `GET/PATCH /v1/me/preferences`,
  `GET/PATCH /v1/orgs/{id}/preferences`,
  `GET /v1/units`. Admin-only paths gated by
  `require_role(Admin)`. All routes derive `utoipa::ToSchema`
  (workspace R7).
- `starter-client-rs` methods + `starter-cli prefs` subcommand
  (`get` / `set` / `units`).
- `iso_currency` lands HERE per D-U0.3 from
  [`starter-prefs-spi`](../starter-prefs-spi/), not on
  `starter-spi`.
- Migrations at `crates/starter-prefs/migrations/0001_starter_prefs.sql`
  per the workspace's namespaced-migration pattern;
  `INTEGER`-typed timestamps for UTC epoch ms per R1.

### Phase 2 — `Accept-Units` middleware (stages 8–10)

- `accept_units_layer(registry, prefs_resolver)` tower Layer
  in `starter-server`. Parses `Accept-Units`, resolves prefs
  once per request, sets `Vary: Accept-Units`, inserts
  `UnitsCtx` into request extensions.
- Per R6 the middleware does **not** mutate response bodies —
  it resolves prefs once and exposes them to typed serialisers
  via `UnitsCtx::convert(quantity, value, source_unit)`.
- Per-series wire shape per R8:
  `SeriesEnvelope<T> { slot, quantity, unit, points: Vec<(i64, T)> }`.
  Metadata is one-per-series, never per-value.
- Canonical-only-logs audit at
  `crates/starter-server/tests/canonical_logs.rs` — the SCOPE
  Smoke-tests block "Canonical-only logs" test, asserting no
  log line contains `"°F"`, `" psi"`, `" mph"`, `" lb"`.
- CDN-cache caveat docs noting that `Vary: Accept-Units` is
  advisory; CloudFront/Fastly/Cloudflare do not key on custom
  headers by default (SCOPE R7 quotes this).

### Phase 3 — `starter-i18n` (stages 12–15)

- New crate at `crates/starter-i18n/` with
  `default-features = []`.
- `parse_accept_language(header)` + `pick_language()` with the
  R5 fallback chain (requested → language family → `en`).
- Catalog format: plain JSON keyed by `MessageKey`,
  `deny_unknown_fields` on the loader. `MessageBundle` walks
  the fallback chain.
- `accept_language_layer(bundle)` tower Layer in
  `starter-server`. Parses `Accept-Language`, picks a
  `LanguageTag`, sets `Content-Language` + `Vary:
  Accept-Language`, inserts `LocaleCtx`. Optional
  `X-I18n-Fallback` header off by default, opt-in per route.
- Catalog fingerprint: sha256 hex prefix, 16 chars (D-PI.5).
- Routes behind a `routes` feature:
  `GET /v1/i18n/manifest` (`{<lang>: <fingerprint>}`),
  `GET /v1/i18n/catalogs/{lang}` (ETag),
  `GET /v1/i18n/catalogs/{lang}-{fingerprint}.json`
  (immutable cache headers).
- Seed catalogs at
  `crates/starter-i18n/catalogs/starter/en.json` and `es.json`,
  compiled in via `include_str!`. `es.json` is a complete
  translation of every `en.json` key (test enforces parity).

### Phase 4 — `@nube/starter-ui-core` additions (stages 17–18)

- `packages/starter-ui-core/src/preferences/`:
  `PreferencesProvider` (react-query for fetch, React context
  for plumb-through, `setPreferences(patch)` callback that
  PATCHes and invalidates).
- `formatters.ts`: pure functions `formatDate`, `formatTime`,
  `formatNumber`, `formatCurrency`, `formatQuantity`. Backed by
  `Intl.*` keyed off resolved prefs; `formatQuantity` does the
  unit-conversion via a static unit table kept in sync with
  `/v1/units`.
- `packages/starter-ui-core/src/i18n/`: `IntlProvider`
  component that fetches `/v1/i18n/manifest` on mount, then the
  fingerprinted catalog URL for `prefs.language`. Wraps
  react-intl's `IntlProvider`. `useTranslate()` hook with a
  typed `MessageKey` overload.
- `SettingsPage` component bound to `PreferencesPatch`; fields
  for every column (timezone via `Intl.supportedValuesOf("timeZone")`,
  language list from manifest, every per-unit selector, theme /
  date / time / week / number / currency). Wired into
  `starter-auth-users`' account-page surface (consumer mounts
  `<SettingsPage />` at `/account/settings`).
- Re-export `PreferencesProvider`, `usePreferences`, the
  formatters, `IntlProvider`, `useTranslate`, `SettingsPage`
  via the package's `exports` map.
- All Phase 4 code lives in the existing
  `@nube/starter-ui-core` package — additive only, no new
  package.

### Phase 5 — Diagnostics rewriter (stage 20)

- Behind a `diagnostics` cargo feature on `starter-i18n`,
  default off.
- `DiagnosticBody` response-extension marker: handlers opt in
  via `response.extensions_mut().insert(DiagnosticBody::new())`.
- A tower Layer reads the extension on the way out and rewrites
  only the documented envelope shape (`{diagnostic: {code,
  params}}`) at the documented top-level paths (locked in
  stage 1).
- Per R5 the rewriter is scope-limited: never walks arbitrary
  JSON, never rewrites SSE / chunked / streaming responses,
  bails on `text/event-stream` and `Transfer-Encoding:
  chunked`.

### Workspace verification (stages 21–22)

- "Headless appliance keeps working" smoke — a binary with
  `starter-auth-token` + `starter-secrets-file` and without
  `starter-prefs` / `starter-i18n` builds and runs.
- "Add a language" smoke — drop `fr.json` in, bump version,
  rebuild; manifest gains the language.
- "Canonical-only logs" smoke (re-run as workspace gate).
- "Australian operator" smoke — full UI + server stack.
- Dep-tree gates: `starter-spi`, `starter-flow-spi` (unchanged),
  `starter-prefs`, `starter-i18n` all match their baselines /
  contain only expected deps.
- R1–R8 grep + cargo-tree + behavioural checks recorded in
  `DOCS/user/scope/PHASES-1-5-VERIFY.md`.

## Out of scope

- **Postgres `PrefsStore` impl.** Deferred per D-PI.2;
  follow-up job once a real consumer needs it.
- **`Repository<T>` derive for prefs tables.** SCOPE
  Open-questions explicitly defers to v0.2; hand-written sqlx
  is fine at this volume.
- **`adk-rust` / `starter-flow` work.** Unrelated SCOPE; this
  job touches `starter-flow-spi` only via the F-0.1 baseline
  diff (which should be zero after feature-gating).
- **Block-defined message keys.** SCOPE Open-questions
  flags this as a future extension; not blocking v1.
- **Currency FX, RTL layout, accessibility prefs, JWT claims
  spec, per-device timezone sync, translation-management
  system, user-authored content translation.** All listed
  Non-goals in the user SCOPE.
- **Hot-reload of catalogs.** The bundle loads at startup;
  changes require a restart. (`include_str!` compiles them in
  for the platform-owned catalogs.)
- **A v2 namespaced-quantities path.** SCOPE Consumer-defined
  quantities (v2) section documents this; not v1.
- **New Quantity / Unit variants.** D-U0.1 / D-U0.2 from Phase
  0 locked the closed-enum membership; no expansion here.
- **`SSR`-aware theme cookie.** SCOPE Decisions-made marks
  this as deferred; `localStorage` is sufficient for SPA paint.

## Hard rules (load-bearing — inherited verbatim from user SCOPE)

- **R1 — Store canonical, convert at the edge.** Every
  timestamp UTC epoch ms; every physical quantity stored in
  canonical SI; money in minor-units + ISO 4217. Conversion
  lives at REST handler / CLI / UI formatter / email
  generator only. Logs are always canonical (stage 10
  smoke).
- **R2 — Display vs quantity prefs.** The resolver returns
  one shape; only quantity prefs feed the serialiser; display
  prefs are forwarded as-is for the client to render.
- **R3 — Three-layer resolution: user → org → default.**
  Per-field independent precedence; `"auto"` follows the
  derivation order block (`unit_system` table → ICU locale
  default → hardcoded). Multi-org case keyed by
  `(user_id, workspace_id)`.
- **R4 — Closed `Quantity` / `Unit` enums.** No new variants
  in this job; the Phase 0 set is the v1 set.
- **R5 — Translation client-side by default.** Server emits
  `{code, params}`; client looks up `code` in its bundle.
  Diagnostics rewriter (Phase 5) is the documented opt-in
  exception, scope-limited.
- **R6 — Conversion at exactly one layer per surface.** REST
  handler / CLI / UI formatter / email — never twice. Logs
  and inter-service RPC are never converted.
- **R7 — Custom `Accept-Units` / `Accept-Language` headers.**
  Not `Accept` media-type parameters. `Vary` advertised; CDN
  caveat documented.
- **R8 — Per-series unit metadata.** Declared once per series,
  never per point.

## Constraints

- **MSRV 1.78** (workspace). `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo fmt --check`
  non-negotiable.
- **`default-features = []` posture stays.** Both new Rust
  crates ship with no default features; the routes / sqlite /
  diagnostics features are opt-in.
- **Workspace policy R5 — minimal-by-default.** The "Headless
  appliance keeps working" smoke is the load-bearing test.
- **Tests live with the code.** Each stage commits its tests
  in the same commit as the body (workspace-wide rule).
- **`starter-flow-spi` baseline unchanged.** F-0.1 closure is
  what makes this constraint honest; if it stays broken after
  stage 1, every later stage's dep-tree check has the wrong
  reference.
- **One logical batch per stage.** 22-stage shape is
  intentional — stage size stays small.
- **No new top-level workspace member outside the two new
  Rust crates.** Phase 4 lives entirely inside the existing
  `packages/starter-ui-core/` package.

## Decisions

Locked in stage 1. Each lists rule-it-derives-from and
revisit-trigger.

### D-PI.1 — `PrefsStore` trait shape

Sqlx::Pool-backed for v1. Trait methods take a `&self` and
internally use a `sqlx::SqlitePool` (the only impl this job
lands). No generic `Repository<T>` derive.

- **Why.** SCOPE Open-questions: *"`Repository<T>` derive for
  prefs tables? Deferred to v0.2 with the rest of the derive
  work in `starter-store-*`. Hand-written sqlx is fine at this
  volume."* Two tables, simple shape.
- **Revisit when.** `starter-store-*` ships the v0.2
  `Repository<T>` derive; at that point `PrefsStore` is a
  candidate first user.

### D-PI.2 — Postgres `PrefsStore` impl posture

**Deferred.** SQLite-only for this job. The SCOPE Crate-layout
block lists `starter-store-sqlite` explicitly; Postgres is
implied by the existing `starter-store-postgres` workspace
member but not by this SCOPE.

- **Why.** Shipping Postgres requires the same migrations,
  same trait impls, and a `migration_template` lint rule per
  the SCOPE block on `INTEGER` vs `BIGINT` for epoch-ms
  columns. Adding it here doubles the migration-side
  surface; a real consumer asking for Postgres is the right
  trigger.
- **Revisit when.** A consumer surfaces a real Postgres prefs
  deployment.

### D-PI.3 — `iso_currency` lands in `starter-prefs`

`iso_currency` is a new dep on `crates/starter-prefs`, not on
`starter-spi`. D-U0.3 from
[`starter-prefs-spi`](../starter-prefs-spi/SCOPE.md) said the
ISO 4217 table is the resolver's concern, not the SPI's.

- **Why.** Pulling `iso_currency` into `starter-spi` would
  drag the static table into every downstream crate's dep
  tree (and into `starter-flow-spi`, which would break F-0.1
  the moment it stops being feature-gated). Containing the
  dep to the crate that actually consults the table is the
  R5 minimal-by-default posture.
- **Revisit when.** A second crate needs ISO 4217 validation
  before this job lands; otherwise it stays in
  `starter-prefs`.

### D-PI.4 — Per-series wire shape

`SeriesEnvelope<T> { slot: String, quantity: Quantity, unit:
Unit, points: Vec<(i64, T)> }` is the structural shape per R8.
Helper struct lives in `starter-prefs` (not `starter-spi`); the
field types it references (`Quantity`, `Unit`) live in
`starter-spi`.

- **Why.** R8 fixes the metadata-hoisting principle; the
  literal tuple shape is documented as the recommended form.
  `starter-prefs` owns the wire helper because that's where
  the rest of the response-side machinery lives.
- **Revisit when.** A consumer surfaces a different
  per-series shape that needs cross-workspace adoption.

### D-PI.5 — Catalog fingerprint algorithm

`sha256(<canonical-JSON bytes>)[..16]` (hex, first 16 chars).
The URL form is
`/v1/i18n/catalogs/{lang}-{fingerprint}.json` for
immutable-cache deployments; the plain form
`/v1/i18n/catalogs/{lang}` returns the same body with an ETag
matching the fingerprint.

- **Why.** Per SCOPE API-surface block: *"`/v1/i18n/manifest`
  — `{ "<lang>": "<fingerprint>" }` map for every shipped
  language, ETag'd."* A truncated sha256 is enough collision
  resistance for a content-hash URL; full sha256 is 64 chars
  which is ugly on the wire.
- **Revisit when.** A real collision shows up in the wild
  (will not happen at the catalog cardinality this carries).

### D-PI.6 — Phase 4 state management

`PreferencesProvider` uses **react-query for fetch + React
context for plumb-through**. Ephemeral UI state (form
in-progress values, dirty flags) lives in a small zustand
store scoped to the Settings page. No global zustand for
preferences.

- **Why.** Matches the existing `@nube/starter-ui-core` posture
  (the package already wraps `@tanstack/react-query` for auth
  and exposes a query module). A new global preferences store
  would fragment state management; react-query's cache + a
  context exposing the resolved view is the workspace's
  established pattern.
- **Revisit when.** Settings page complexity outgrows a single
  form (e.g. a multi-step preferences wizard) — at that point
  a scoped zustand store is fine; the global one stays
  unnecessary.

### D-PI.7 — Phase 5 rewriter top-level paths

The rewriter touches the **response root only** when the
`DiagnosticBody` extension is present. Specifically: if the
response body is a JSON object whose top-level shape is
`{diagnostic: {code, params}}`, the `code` is resolved via the
LocaleCtx bundle and the body becomes
`{diagnostic: {code, params, translated_text}}`. Other shapes
are passed through unchanged.

- **Why.** SCOPE R5: *"The rewriter is scope-limited: it does
  not walk arbitrary JSON bodies. It runs only when the
  handler opts in by inserting a DiagnosticBody extension on
  the response, and it touches only the declared envelope
  shape (`{diagnostic: {code, params}}`) at the documented
  top-level paths."* Defining "documented" as "the response
  root" is the smallest scope that satisfies the SCOPE.
- **Revisit when.** A consumer surfaces a need for nested
  diagnostic envelopes (e.g. a batch response carrying N
  per-item diagnostics). At that point the rewriter grows a
  per-item walk gated on a different opt-in marker; the
  current top-level path stays default.

## Cross-cutting checks the runner must keep honest

- **R1 logs canonical** — stage 10 audit (re-run at stage 21
  as a workspace gate).
- **R2 display vs quantity** — handler tests cover that
  display prefs flow through to the response untouched while
  quantity prefs feed `UnitsCtx::convert`.
- **R3 resolver precedence** — stage 4 resolver tests cover
  the three SCOPE Smoke-tests-block cases word-for-word.
- **R4 closed enums** — `grep -rn '#\[non_exhaustive\]'
  crates/starter-spi/src/units crates/starter-spi/src/preferences`
  returns zero hits at stage 21.
- **R5 client-side default** — Phase 4 catalog lookup proves
  this; Phase 5 rewriter is feature-gated default-off.
- **R6 conversion at one layer** — stage 8 + stage 17
  formatters; no double-convert path. Stage 10 log audit is
  the negative check.
- **R7 custom headers** — `Vary: Accept-Units` and
  `Vary: Accept-Language` set on every response that runs
  through the middleware; CDN caveat documented in
  `DOCS/user/scope/`.
- **R8 per-series metadata** — `SeriesEnvelope<T>` round-trip
  test at stage 9.
- **`starter-flow-spi` baseline unchanged** — stage 1
  closure + stage 21 re-confirm.
- **Headless appliance builds** — stage 21 structural test.
- **`@nube/starter-ui-core` typecheck + test** — stage 18 +
  stage 21.

## Deliverables

- `crates/starter-prefs/` populated (Cargo.toml, lib.rs,
  resolver.rs, store.rs, routes.rs behind `routes`,
  middleware.rs behind `routes`, migrations/, tests/).
- `crates/starter-i18n/` populated (locale.rs, catalog.rs,
  bundle.rs, translate.rs, platform.rs, routes.rs behind
  `routes`, middleware.rs behind `routes`, catalogs/starter/
  with en.json + es.json, tests/).
- `starter-server` gains the two tower middlewares + the
  canonical-only-logs audit.
- `starter-client-rs` gains the preferences client methods.
- `starter-cli` gains the `prefs` subcommand.
- `packages/starter-ui-core/` gains `src/preferences/`,
  `src/i18n/`, and the `SettingsPage` export.
- `DOCS/user/scope/capture-baseline.sh` committed.
- `DOCS/user/scope/starter-spi-deps.baseline.txt` updated for
  the feature-gated state.
- `DOCS/user/scope/PHASES-1-5-VERIFY.md` committed (analog of
  PHASE0-VERIFY.md from the prior job).
- All workspace gates green (cargo / clippy / fmt / cargo-tree
  / pnpm test / pnpm typecheck).

## Open questions (resolve in stage 1)

1. **`PrefsStore` Postgres impl in this job?** Bias: no.
   SCOPE Crate-layout does not mandate it; landing here
   doubles surface.
2. **Timestamp wire form on `ResolvedPreferences::updated_at`.**
   Bias: `i64` UTC epoch ms (R1). Stage 4 / 5 confirm.
3. **react-intl version on `@nube/starter-ui-core`.** Bias:
   pin to the major already in the existing dep tree if
   present; otherwise the latest stable `^6`. Stage 1
   `pnpm why` check.
4. **`Accept-Units: preferred` vs unset.** Bias: unset
   defaults to `preferred` (the SCOPE R7 block reads as
   "preferred is the default"); MCP / programmatic callers
   set `canonical` explicitly. Stage 8 fixes the behaviour.
5. **`SettingsPage` mount path.** Bias: `/account/settings`
   in the consumer's router; the consumer wires the component
   themselves. SCOPE Phase 4 block says "wired into
   starter-auth-users' account page surface" — interpret as
   "exposed via export so consumer can mount", not "starter
   ships a hardcoded route."

D-PI.1 through D-PI.7 are locked in stage 1; the five above
are sub-decisions whose outcome lands under "Decisions" once
stage 1 finishes.
