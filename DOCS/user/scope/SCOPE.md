# starter-prefs + starter-i18n — Scope

## One-line summary

Two small, independently-useful crates (`starter-prefs`,
`starter-i18n`) plus thin TS counterparts (preferences hooks +
formatters in `@nube/starter-ui-core`, message catalogs alongside)
that give every starter-based product a working **user-preferences +
i18n backend** out of the box: locale, language, timezone, unit
system, per-quantity unit overrides, date/time/number formats,
currency, and theme — with a three-layer (user → org → default)
resolution model, canonical-only storage, and edge-of-API conversion.

The two crates are split because they have different scope and
different dependency profiles. `starter-prefs` owns the persisted
preference model and its REST surface; `starter-i18n` owns translation
bundles + locale resolution. A consumer can use either alone.

## Why this exists

Every starter-based product — whether a single-operator headless
appliance running `starter-auth-token` or a multi-user dashboard
running `starter-auth-users` — has a user (or, at minimum, an
operator) somewhere, and that user is not necessarily American, not
necessarily English-speaking, not necessarily on metric or imperial,
and not necessarily in UTC. Shipping a US-centric API and retrofitting
i18n later is a known multi-year trap: it spreads through every
endpoint that emits a timestamp, every chart that renders a number,
every email or PDF the server generates.

Across past Rubix work — see the prior-art reference design at
`rubix-agent/docs/design/user/USER-PREFERENCES.md` and the reference
Rust shape at `rubix-agent/crates/domain-i18n` — we have already
learned the load-bearing shape of the problem:

- Locale, language, and timezone are **three separate things**, often
  conflated; each one drives a different rendering path.
- Per-unit overrides matter more than a single `unit_system` flag —
  Australians want metric everything except °F on the BBQ; UK users
  want metric weather and imperial road signs.
- The data model is "**store canonical, convert at the edge**" —
  every physical quantity lives in SI in the database; the wire/UI
  converts to the caller's preferred unit on read.
- Translation is **client-side by default** (the server emits stable
  message keys + params, the client picks a bundle), with one
  documented exception for structured server-originated diagnostics
  where the server must translate so every transport renders verbatim.

Starter ships this as crates so a new product gets it for free, on the
same R1–R8 rules as the rest of the workspace.

## Relationship to existing crates

```
starter-spi                          (trait seams + DTOs)
   ↑
   ├── starter-prefs ──────→ depends on starter-spi only
   │                          (Preferences DTOs, Quantity / Unit
   │                           enums, UnitRegistry trait live in
   │                           starter-spi; the crate implements the
   │                           resolver + REST surface + storage glue)
   │
   ├── starter-i18n ──────→ depends on starter-spi only
   │                          (LanguageTag, Catalog, MessageBundle;
   │                           pure domain logic, no HTTP, no I/O)
   │
   ├── starter-server ──→ optional dep on starter-prefs / starter-i18n
   │                       behind cargo features; mounts the
   │                       `/v1/me/preferences`, `/v1/orgs/{id}/
   │                       preferences`, `/v1/units`, and
   │                       `/v1/i18n/catalogs/{lang}` routes and the
   │                       `Accept-Units` / `Accept-Language` layers
   │
   └── starter-store-* ──→ ships the migrations for the preference
                            tables under namespaced sources
                            (`starter_prefs`, `starter_i18n`); no SQL
                            outside these crates per R4 of the
                            workspace SCOPE
```

`starter-prefs` and `starter-i18n` are **strictly optional** (workspace
R5). Default-features stay empty; a consumer who doesn't want
preferences or i18n pays nothing — no migrations registered, no routes
mounted, no extra deps pulled.

Auth coupling: `starter-prefs` reads the caller's identity through the
`Principal` already produced by whichever `Authenticator` the binary
wires in (`starter-auth-token`, `starter-auth-users`,
`starter-auth-oauth`, or a consumer impl). It is **not** an auth
crate — it does not know how the caller was authenticated, only that
a `Principal` is available.

## Hard rules (load-bearing)

These rules are why the two crates compose cleanly with the rest of
starter. Break one and the design slips.

### R1 — Store canonical, convert at the edge

- Every timestamp column is **UTC epoch milliseconds** (`INTEGER`) or
  ISO-8601 with `Z`. Never a local-time column, ever. Never a
  TZ-offset sidecar — timezone belongs on the user/event, not on the
  column.
- Every physical quantity is stored in its **canonical SI unit** (°C,
  kPa, L/s, m/s, kWh, …). Conversion to the user's preferred unit
  happens at the presentation edge (REST serialiser, CLI formatter,
  React formatter), never in storage.
- Money is **minor-units integer + ISO 4217 code**. Never floats. No
  implicit currency.
- Inter-service / inter-process traffic stays canonical. Only the
  human-facing edge formats.

The rule applies to starter's own code. Consumer code that writes
into the same database is free to choose its own conventions — but
the helpers, formatters, and middleware starter ships only know about
canonical storage.

### R2 — Display-only prefs vs quantity prefs

Some preferences (timezone, unit system, per-unit overrides, currency)
participate in value conversion. Others (date format, time format,
week start, number format, theme) are purely display-side rendering
choices with no canonical-storage counterpart. Both are stored in the
same row and resolved through the same three-layer model, but only
the first group ever feeds the serialiser. The second group is
forwarded to clients in the resolved view and rendered there.

### R3 — Three-layer resolution: user → org → default

Preferences resolve inside-out: a non-null user value overrides a
non-null org value, which overrides the system default. The default
is hardcoded in `starter-spi` (`en-US`, `UTC`, metric, ISO date, 24h,
`1,234.56` number format, `system` theme) so the resolved view
**never** returns `null` or `"auto"` — clients always see concrete
values.

**Precedence within a single field.** For each individual column the
resolver picks the first non-null layer (`user` → `org` → default). It
does **not** cross-reference other columns. The cross-column
interaction (`unit_system` vs `temperature_unit`) is handled inside
the `"auto"` resolution rule below — never by overlaying one column on
top of another.

**`"auto"` semantics and precedence.** Where present, `"auto"` is a
placeholder meaning *derive*. The derivation order for a per-unit
field (`temperature_unit`, `pressure_unit`, `speed_unit`,
`length_unit`, `mass_unit`) is:

```
explicit value at any layer
  → unit_system at the same/closer layer (metric / imperial table below)
  → locale-derived default (ICU, where one exists)
  → hardcoded system default
```

For `currency: "auto"`: derive from `locale` via `iso_currency`'s
locale → currency table (e.g. `en-AU` → `AUD`, `en-GB` → `GBP`).

For `date_format`, `time_format`, `number_format`, `week_start` set to
`"auto"`: defer to ICU's locale defaults (`Intl.DateTimeFormat`
patterns, `Intl.NumberFormat` grouping, `firstDayOfWeek` per the CLDR
data ICU ships — Saturday-start locales such as `ar-SA` are picked up
automatically). The hardcoded system default only fires when ICU has
no opinion for the resolved locale.

The `unit_system → unit` mapping (the table the resolver consults
when a per-unit field is `"auto"` and `unit_system` is set):

| Quantity      | `unit_system: metric` | `unit_system: imperial` |
|---|---|---|
| temperature   | `C`                   | `F`                     |
| pressure      | `kPa`                 | `psi`                   |
| speed         | `km/h`                | `mph`                   |
| length        | `m`                   | `ft`                    |
| mass          | `kg`                  | `lb`                    |

The BBQ case from the intro is the explicit reason for this layering:
an Australian sets `unit_system: metric` at the org layer and
`temperature_unit: F` at the user layer — the per-unit override wins
because it's the more specific layer for that one column.

Multi-org case: a user belongs to N orgs and has one row per
`(user_id, workspace_id)` in `user_preferences`. The active org for a
request comes from the `Principal` (or an explicit `?org=` query
param on `GET /v1/me/preferences`). Switching orgs switches the
entire preference context.

Single-tenant binaries (most `starter-auth-token` deployments) skip
the org layer entirely — `workspace_id` defaults to a reserved
sentinel `"@starter/default"` and `org_preferences` has at most one
row. The leading `@` is reserved by starter; consumer-created org ids
must not start with `@`, which is enforced by the org-creation path
in `starter-auth-users`.

### R4 — One source of truth for the unit registry; closed enums

`starter-spi` owns the `Quantity` and `Unit` enums and the
`UnitRegistry` trait + `StaticRegistry` impl. The enums are
**closed** — extensions cannot add variants — because every wire
identifier and every UI label must be known to the platform. New
quantities or units land via PR on `starter-spi`; the friction is
intentional and matches workspace R8 (small public surface, slow
changes).

Conversion factors are delegated to `uom` internally. The registry is
the thin serialisable veneer; we never hand-write conversion math.

Versioning rules:

- Adding a `Quantity` or `Unit` variant is backward-compatible.
- Renaming or removing requires a major version bump on `starter-spi`
  plus a deprecated alias for at least one major.
- Changing a quantity's canonical unit is high-cost (major bump +
  backfill migration); choose canonical units carefully up front.

`GET /v1/units` exposes the registry to clients with an ETag and an
`X-Platform-Version` header so the TS side can render unit pickers
without hardcoding.

### R5 — Translation is client-side by default

`starter-i18n` ships the **server-side machinery** (catalog loading,
locale parsing, `Accept-Language` resolution, fallback chains) but
the default rendering path is: server emits
`{ code: "flow.error", params: { node: "x" } }`, client looks up
`flow.error` in its bundle, renders. Backend stays language-neutral;
adding a translation does not require a backend deploy.

The single documented exception is **structured server-originated
diagnostics** (long-running async jobs, audit-trail messages,
scheduled exports / emails that have no client to translate them).
The rewriter is **scope-limited**: it does not walk arbitrary JSON
bodies. It runs only when the handler opts in by inserting a
`DiagnosticBody` extension on the response, and it touches only the
declared envelope shape (`{ diagnostic: { code, params } }`) at the
documented top-level paths. Streaming responses (SSE, chunked) are
**not** rewritten — they emit codes + params and the consumer
translates per event. Static UI chrome (sidebar labels, button text,
page titles) always stays client-side.

Fallback chain: requested language → language family (`zh-TW` →
`zh`) → `en`. Missing keys fall through to the source string, never
error. **Observability:** every fallback emits a `tracing::debug!`
event and the response carries an opt-in `X-I18n-Fallback: <lang>`
header (off by default; enable per route or globally via
`accept_language_layer().with_fallback_header(true)`) so dev/staging
catches missing translations without breaking the no-error guarantee.

### R6 — Conversion happens at exactly one layer per surface

| Surface | Converts where |
|---|---|
| REST API | Handler / response serialiser via the `UnitsCtx` that `accept_units_layer` puts on the request. The middleware **does not mutate response bodies** — it resolves prefs once per request and exposes them to typed serialisers; conversion is opt-in per response per R8. |
| CLI | Client-side, using prefs fetched once per session via `starter-client-rs` |
| Studio / admin UI | Client-side formatter (`@nube/starter-ui-core`) |
| Logs / audit trail | **Never converted** — always canonical UTC + SI |
| Inter-service RPC | **Never converted** — canonical only |
| Server-originated emails / scheduled exports | Server-side, re-resolved from DB at send time (never a cached JWT claim) |

### R7 — Content negotiation via dedicated headers

- `Accept-Units: preferred` (default) — middleware converts.
- `Accept-Units: canonical` — MCP / programmatic; no conversion;
  responses carry stable quantity/unit codes.
- `Accept-Language: en-AU, en;q=0.9, *;q=0.5` — standard BCP-47;
  `starter-i18n` parses this and picks a bundle.

Responses always set `Vary: Accept-Units, Accept-Language` so caches
key correctly. Responses set `Content-Language` to the language
actually used (post-fallback) so clients can detect when they fell
back to English.

Custom header (not a media-type parameter on `Accept`) because most
CDNs collapse media-type parameters when keying cache.

**Operator note: `Vary` is advisory.** CloudFront, Fastly, and
Cloudflare do **not** key on `Accept-Units` (or any custom header) by
default — `Vary` alone is insufficient. Deployments that put a CDN in
front of the server must either explicitly add `Accept-Units` and
`Accept-Language` to the cache key in their edge config, or mark
unit/locale-sensitive responses uncacheable. Starter cannot enforce
this; the docs flag it because the failure mode (one user's units
served to another) is silent and bad. The custom-header design still
wins over an `Accept` media-type parameter — most edges support
varying on a named header once configured, but few support varying on
media-type parameters at all.

### R8 — Per-series unit metadata, not per-value

Timeseries responses declare `quantity` and `unit` **once per series**,
not per row:

```json
{ "series": [{
  "slot": "temp_in",
  "quantity": "temperature",
  "unit": "fahrenheit",
  "points": [[1713456000000, 72.4], [1713456060000, 72.6]]
}] }
```

Single-value reads use the inline form `{ "value": 72.4, "unit":
"fahrenheit", "quantity": "temperature" }`. The rule: unit + quantity
metadata are declared once at the tightest scope that covers
homogeneous values.

The `points: [[ts, value], …]` tuple-of-arrays shape is **new in this
crate** — the rest of the workspace has no shared timeseries
convention yet. If a consumer's timeseries surface differs, the rule
that matters is the metadata-hoisting principle (declared once at the
tightest homogeneous scope), not the literal tuple shape.

## Preferences model

```
starter_prefs_org
  workspace_id     TEXT PRIMARY KEY
  timezone         TEXT      -- IANA, e.g. "Australia/Brisbane"
  locale           TEXT      -- BCP-47, e.g. "en-AU"
  language         TEXT      -- BCP-47 lang subtag, e.g. "en"
  unit_system      TEXT      -- "metric" | "imperial"
  temperature_unit TEXT      -- "C" | "F" | "auto"
  pressure_unit    TEXT      -- "kPa" | "psi" | "bar" | "auto"
  speed_unit       TEXT      -- "m/s" | "km/h" | "mph" | "knot" | "auto"
  length_unit      TEXT      -- "m" | "ft" | "auto"
  mass_unit        TEXT      -- "kg" | "lb" | "auto"
  date_format      TEXT      -- "auto" | "YYYY-MM-DD" | "DD/MM/YYYY" | "MM/DD/YYYY"
  time_format      TEXT      -- "auto" | "24h" | "12h"
  week_start       TEXT      -- "auto" | "monday" | "sunday"
  number_format    TEXT      -- "auto" | "1,234.56" | "1.234,56" | "1 234,56"
  currency         TEXT      -- ISO 4217 or "auto"
  updated_at       INTEGER   -- UTC epoch ms; BIGINT in Postgres
                              -- migrations (SQLite INTEGER is 64-bit;
                              -- Postgres INTEGER is 32-bit and would
                              -- overflow in 2038)

starter_prefs_user
  user_id          TEXT
  workspace_id     TEXT
  -- Same columns as starter_prefs_org, all NULLABLE.
  -- NULL means "inherit from org".
  -- Plus user-only fields with no org counterpart:
  theme            TEXT      -- "light" | "dark" | "system"
  updated_at       INTEGER
  PRIMARY KEY (user_id, workspace_id)
```

Resolution: `user_value ?? org_value ?? system_default`. The resolver
in `starter-prefs::resolve` returns a fully-populated
`ResolvedPreferences` struct — no `Option`, no `"auto"` — so callers
never have to think about NULL semantics.

### What `currency` actually drives

`currency` is a **display + authoring default**, not a conversion
input — FX is a non-goal. Specifically:

- It is the default currency stamped on newly-authored money values
  in the consumer's domain (the consumer reads it via
  `prefs.currency` when constructing a new entity).
- It is the symbol-style used when rendering money via
  `formatCurrency(amount, prefs)` — i.e. `formatCurrency` accepts an
  amount **with its own currency code** and only consults
  `prefs.currency` to pick a fallback for amounts that lack one.
- It is **never** used to convert one currency to another. A money
  value's declared code always wins over the user's pref.

A consumer who has no money in their domain can ignore the column;
the resolver still populates a value (defaulted from locale), but
nothing reads it.

### Where time columns store milliseconds

All `INTEGER`-typed time columns in starter-owned tables (`updated_at`
above and any future `created_at`, `last_used_at`, etc.) store **UTC
epoch milliseconds**. On SQLite the column type is `INTEGER` (64-bit
affinity). On Postgres the migration emits `BIGINT` — `INTEGER` is
32-bit and rolls over in 2038. The `starter-store-postgres` migration
template enforces this with a check during migration linting.

## API surface

`starter-prefs` mounts (behind the `routes` feature):

- `GET /v1/me/preferences?org=<workspace_id>` — resolved view for the
  given org. `org` defaults to the active org on the `Principal`.
- `PATCH /v1/me/preferences?org=<workspace_id>` — update the user
  layer for that org. Fields set to `null` revert to inherit.
- `GET /v1/orgs/{id}/preferences` — org layer, admin-only via
  `require_role(Admin)`.
- `PATCH /v1/orgs/{id}/preferences` — admin-only.
- `GET /v1/units` — public quantity/unit registry as JSON with ETag.
- `GET /v1/i18n/catalogs/{language}` — translation bundle for one
  language (mounted by `starter-i18n` when its `routes` feature is
  on); cacheable with ETag. For long-lived edge caching the catalog
  body is also addressable by a content-hash URL
  (`/v1/i18n/catalogs/{language}-{fingerprint}.json`) emitted by
  `GET /v1/i18n/manifest`.
- `GET /v1/i18n/manifest` — `{ "<lang>": "<fingerprint>" }` map for
  every shipped language, ETag'd. Clients fetch the manifest once at
  boot, then fetch each language at the fingerprinted URL so the
  bundles can be cached `immutable` for years. Skipping the manifest
  and going straight to `/v1/i18n/catalogs/{lang}` is supported (the
  ETag path still works); the manifest exists for CDN-heavy
  deployments.

All routes derive `utoipa::ToSchema` so they appear in the
auto-generated `openapi.json` per workspace R7.

### Middleware

Two thin tower layers ship in `starter-server` and read prefs/locale
off the request:

- `accept_units_layer(registry, prefs_resolver)` — inspects
  `Accept-Units`, resolves the caller's prefs once per request, sets
  `Vary: Accept-Units`, and inserts a `UnitsCtx` into request
  extensions. Handlers (or response serialisers built on
  `starter-spi`'s DTO traits) call `UnitsCtx::convert(quantity,
  value, source_unit)` instead of emitting raw values.
- `accept_language_layer(bundle)` — parses `Accept-Language`, picks a
  language with fallback, sets `Content-Language` + `Vary:
  Accept-Language`, inserts a `LocaleCtx` into request extensions.
  The diagnostics rewriter (off by default; opt-in) runs on the way
  out.

Both layers are no-ops when the relevant crate isn't compiled in.

## Library choices

| Concern | Crate | Notes |
|---|---|---|
| Timezone-aware datetime | `jiff` | IANA tz built in. Preferred over chrono / time. |
| Locale parsing | `icu_locale` | BCP-47 parsing + fallback chain. |
| Number / date formatting (server-side rare path) | `icu_datetime`, `icu_decimal` | Used only for server-originated emails / exports. The browser owns the hot path. |
| Unit conversion | `uom` | Type-safe SI units, compile-time dimensional analysis. |
| Translation bundles | JSON catalogs, ICU MessageFormat | Same shape Rubix `domain-i18n` uses. No Fluent in v1 — `react-intl` on the TS side, plain ICU strings on the Rust side. Revisit if plural / gender rules outgrow ICU. |
| ISO 4217 currency codes | `iso_currency` | Static table; no FX. |

Principle: ICU4X for presentation, `jiff` for time, `uom` for units,
plain JSON catalogs for translations. Don't wrap unnecessarily.

## TypeScript surface

Per workspace R6 (TS client has zero React; UI-kit has zero I/O;
UI-core owns the brain) the prefs + i18n surface splits as follows.

### `@nube/starter-client-ts`

Generated from `openapi.json` per workspace R7. Adds:

- Zod schemas: `ResolvedPreferencesSchema`, `PreferencesPatchSchema`,
  `UnitRegistryDtoSchema`.
- Methods: `getMyPreferences(orgId?)`, `patchMyPreferences(orgId,
  patch)`, `getOrgPreferences(orgId)`, `patchOrgPreferences(orgId,
  patch)`, `getUnits()`, `getI18nCatalog(language)`.

No React, no caching, no hooks — just typed `fetch` wrappers.

### `@nube/starter-ui-kit`

Visual-only. Adds:

- `<ThemeProvider>` (already exists) gains the ability to read
  `theme` from a context provided by `ui-core`. No new I/O.
- Tailwind preset gains tokens for relative-time strings ("just now",
  "X minutes ago") so consumers can theme them.

No prefs hooks, no formatters that fetch.

### `@nube/starter-ui-core`

The brain. Adds:

```
src/preferences/
  PreferencesProvider.tsx     <- React Query + ETag caching
  usePreferences.ts           <- read the resolved prefs
  useUpdatePreferences.ts     <- mutate with optimistic update
  formatters.ts               <- pure functions (no hooks, no React):
                                   formatDate(ts, prefs)
                                   formatTime(ts, prefs)
                                   formatDateTime(ts, prefs)
                                   formatRelativeTime(ts, prefs)
                                   formatNumber(n, prefs)
                                   formatUnit(value, quantity, unit, prefs)
                                   formatCurrency(amount, prefs)

src/i18n/
  IntlProvider.tsx            <- wraps react-intl; reads prefs.language
  useTranslate.ts             <- thin wrapper over useIntl()
  registerExtensionMessages.ts <- block authors register catalogs

src/i18n/catalogs/
  en.json                     <- starter-owned strings (auth UI, errors)
  es.json                     <- seed translations; more added on demand
```

Formatters are **pure functions** with no React dependency, powered
by the browser's built-in `Intl` APIs (zero extra deps). Unit
conversion factors come from `GET /v1/units` (cached with ETag), not
a hardcoded table — the server is the single source of truth.

Query keys are namespaced `['starter', 'prefs', ...]` per workspace
R6.

## Conversion: how a read works end-to-end

1. Sensor writes `72.4 °F` to a slot whose schema declares
   `quantity: Temperature, sensor_unit: Fahrenheit`. The ingest path
   normalises to canonical: `22.44 °C` stored.
2. REST handler queries the slot. The handler emits the canonical
   value in a column-oriented response — `unit` declared at the
   series level, never per-row.
3. `accept_units_layer` resolves the caller's `temperature_unit` pref
   (say, `"F"`), calls
   `registry.convert(Temperature, 22.44, Celsius, Fahrenheit) =
   72.4`, and rewrites the series' `unit` to `"fahrenheit"`.
4. Studio renders `72.4 °F` via `formatUnit(72.4, "temperature",
   "fahrenheit", prefs)` — which is a no-op convert (since the wire
   value already matches the user's pref) plus a localised symbol.

MCP / LLM consumers send `Accept-Units: canonical` and skip step 3:
they get `22.44` with `"unit": "celsius"` and a stable quantity code.

## Crate layout

```
crates/
  starter-spi/                <- (already exists; ADD)
                                 preferences::{
                                   ResolvedPreferences, PreferencesPatch,
                                   Theme, DateFormat, TimeFormat,
                                   WeekStart, NumberFormat, UnitSystem,
                                 }
                                 units::{
                                   Quantity, Unit, QuantityDef,
                                   UnitRegistry, StaticRegistry,
                                   normalize_for_storage,
                                 }
                                 i18n::{
                                   LanguageTag, MessageKey,
                                   Diagnostic, DiagnosticParam,
                                 }

  starter-prefs/              <- NEW. Default-features = [].
    src/
      lib.rs
      resolver.rs             <- 3-layer resolution; pure function
      store.rs                <- trait PrefsStore; impls live behind
                                 features (sqlite, postgres) and reuse
                                 the typed building blocks from
                                 starter-store-*
      routes.rs               <- axum router (behind `routes` feature)
      middleware.rs           <- accept_units_layer
    migrations/
      0001_starter_prefs.sql  <- ships under source = "starter_prefs"
                                 via the namespaced migration runner

  starter-i18n/               <- NEW. Default-features = [].
    src/
      lib.rs
      locale.rs               <- LanguageTag, Accept-Language parser
      catalog.rs              <- JSON catalog format + loader
      bundle.rs               <- MessageBundle with fallback chain
      translate.rs            <- diagnostic post-processor
      platform.rs             <- compiled-in starter-owned catalogs
      routes.rs               <- GET /v1/i18n/catalogs/{lang}
      middleware.rs           <- accept_language_layer
    catalogs/
      starter/
        en.json
        es.json               <- seed; more on demand
```

Both crates land with the same R1–R8 ceilings as the rest of the
workspace: ≤ 400 lines per file, ≤ ~10 public items per module, no
`utils` / `helpers` / `common` modules.

## Decisions made

- **Two crates, not one.** Preferences and i18n have different scope,
  different dep profiles (preferences pulls `uom` + `iso_currency`;
  i18n pulls `icu_locale`), and different opt-in stories. Bundling
  them would force consumers who only want one to pay for both.
- **Closed `Quantity` / `Unit` enums** in `starter-spi`. Extensions
  cannot add variants; new quantities go via platform PR. Friction is
  intentional — a quantity is part of the public data model.
- **Translation defaults to client-side.** Backend stays
  language-neutral. The diagnostics rewriter exception is opt-in.
- **Wire format for timestamps is UTC epoch milliseconds.** Always.
  No second representation, no offset sidecars.
- **`Accept-Units` is a dedicated header.** Not an `Accept` media-
  type parameter. CDN-safe.
- **Theme lives in prefs.** It is a user-only field (no org
  fallback). Client also persists it to `localStorage` so cold-start
  paint is correct before the network round-trip. **SSR is out of
  scope for v1** — `localStorage` is sufficient for SPA paint;
  consumers shipping SSR will want a `starter_theme` cookie set by
  `useUpdatePreferences` and read on the server, but the cookie path
  is not built into v1.
- **`react-intl` on TS, plain ICU JSON on Rust.** Not Fluent in v1 —
  the browser already has ICU via `Intl`, and `react-intl` is the
  de-facto React standard.
- **Single canonical for ratios is 0.0–1.0.** `Percent` is a display
  unit only; the registry rejects `quantity: Ratio, unit: Percent` in
  a slot schema.

### Phase 0 wire boundary (locked)

The closed-enum boundary below is the contract `starter-spi` ships in
Phase 0. R4 applies — adding a variant is a PR on `starter-spi` and
the friction is intentional. Each rule lists the revisit trigger that
would justify reopening it.

- **`Quantity` v1 variants: `Temperature`, `Pressure`, `Speed`,
  `Length`, `Mass`.** These are exactly the physical quantities the
  Preferences model has a column for. `Money` stays out of `Quantity`
  (R1 separates it as `i64` minor-units + ISO 4217 code). `Ratio` is
  **deferred** — no Preferences column needs it in v1; `Percent`
  remains display-only and the rejection rule for
  `{Ratio, Percent}` lands in the Phase 1 resolver once `Ratio` is
  introduced. *Revisit trigger:* a Preferences-model field, a slot
  schema in a shipped block, or a consumer crate surfaces a need for
  a quantity not in this list (energy, power, volume, time-duration,
  angle, ratio, …).
- **`Unit` v1 variants: `C`, `F`, `Kpa`, `Psi`, `Bar`, `MPerS`,
  `KmPerH`, `Mph`, `Knot`, `M`, `Ft`, `Kg`, `Lb`.** Exactly the units
  named in the Preferences-model column comments. The canonical
  storage units from R1 (`°C`, `kPa`, `m/s`, `m`, `kg`) are all in
  this set. *Revisit trigger:* same as `Quantity` — a new column,
  block slot, or consumer need.
- **Canonical SI per `Quantity` (R1):** `Temperature → C`,
  `Pressure → Kpa`, `Speed → MPerS`, `Length → M`, `Mass → Kg`.
  `normalize_for_storage(Quantity, Unit, f64) -> f64` lives in
  `starter-spi::units` and is the single source of truth.
  *Revisit trigger:* never under R1; would require a workspace-rule
  change.
- **Money is not in `Unit`.** Money values are `i64` minor-units plus
  an ISO 4217 currency code (`String` or a dedicated type), per R1.
  *Revisit trigger:* never under R1.
- **`Theme` variants: `Light`, `Dark`, `System`. User-only.** No org
  fallback row; the field is omitted from the org-layer Preferences
  DTO. *Revisit trigger:* a consumer needs org-enforced theming
  (brand lockdown).
- **`DateFormat` variants: `Auto`, `IsoYMD` (`YYYY-MM-DD`),
  `DmySlash` (`DD/MM/YYYY`), `MdySlash` (`MM/DD/YYYY`).** *Revisit
  trigger:* a locale ships that none of these three formats fit
  (e.g. dot-separated `DD.MM.YYYY` for de-DE if `Auto` derivation
  proves insufficient).
- **`TimeFormat` variants: `Auto`, `H24`, `H12`.** *Revisit
  trigger:* none expected.
- **`WeekStart` variants: `Auto`, `Monday`, `Sunday`.** *Revisit
  trigger:* a locale that starts the week on Saturday surfaces as a
  shipped requirement (parts of MENA).
- **`NumberFormat` variants: `Auto`, `CommaDot` (`1,234.56`),
  `DotComma` (`1.234,56`), `SpaceComma` (`1 234,56`).** *Revisit
  trigger:* Indian grouping (`1,23,456.78`) becomes a shipped
  requirement.
- **`UnitSystem` variants: `Metric`, `Imperial`.** Drives `Auto`
  derivation for per-unit fields. *Revisit trigger:* none expected;
  US-customary vs Imperial nuance is handled in the derivation
  table, not as a third variant.

### Phase 1-5 decisions (locked)

Locked in the `starter-prefs-i18n` job, stage 1. Each decision lists
the rule it derives from and the trigger that would justify reopening
it (per the WORKFLOW convention).

**Phase 0 closure**

- **D-0.1 — `starter-spi` deps `uom` + `icu_locale_core` move behind
  default-off cargo features.** Adopt PHASE0-VERIFY.md recommendation
  (b). New features: `units` (gates `uom` + the `starter_spi::units`
  module) and `i18n` (gates `icu_locale_core` + the `starter_spi::i18n`
  module). Defaults stay `["broadcast"]` — no `units`, no `i18n`.
  `starter-prefs` enables `units`; `starter-i18n` enables `i18n`.
  *Derives from:* workspace R5 ("default-features minimal"); SCOPE R4
  ("starter-flow-spi baseline holds across Phase 0"). *Revisit
  trigger:* a future starter crate needs the `Quantity` / `Unit` /
  `LanguageTag` types without opting in via cargo feature — at which
  point the gate should be reconsidered (likely by splitting the
  pure-Rust types from the third-party adapter layer).
- **D-0.2 — `starter-spi-deps.baseline.txt` re-captures against the
  default-feature build** (`cargo tree -p starter-spi --edges normal`,
  no `--all-features`). `uom`, `icu_locale_core`, and their
  transitives (`displaydoc`, `litemap`, `tinystr`, `writeable`,
  `num-traits`, `typenum`) drop out of the baseline.
  `starter-flow-spi-deps.baseline.txt` stays as-is — F-0.1 closes
  because the regression no longer exists. *Derives from:* D-0.1.
  *Revisit trigger:* D-0.1 reopens.
- **D-0.3 — Baseline capture script lives at
  `DOCS/user/scope/capture-baseline.sh`.** Strips worktree paths from
  the top-level crate lines so the file is portable across
  `.codeless/worktrees/job-*` checkouts and CI nodes. The
  `starter-flow-spi` baseline gets the same treatment in a separate
  follow-up (tracked in TODO.md F-0.2 — closed once both baselines
  re-captured through the script). *Derives from:* the
  "byte-for-byte CI gate" posture in Phase 0's verification.
  *Revisit trigger:* a baseline embeds a second worktree-sensitive
  field that the current sed isn't handling.

**Phase 1 — `starter-prefs`**

- **D-1.1 — `PrefsStore` trait is sqlx::Pool-backed, not
  `Repository<T>`.** Concrete shape: `trait PrefsStore { async fn
  load_user(...); async fn load_org(...); async fn upsert_user(...);
  async fn upsert_org(...); }` with a `SqlitePrefsStore` impl that
  holds a `sqlx::SqlitePool`. *Derives from:* SCOPE Open-questions
  block ("`Repository<T>` derive for prefs tables — deferred to
  v0.2"); workspace R5 (minimal surface). *Revisit trigger:*
  `starter-store-*` ships a `Repository<T>` derive consumers can
  reuse without bespoke trait plumbing.
- **D-1.2 — sqlite only for v1; Postgres deferred.** `starter-prefs`
  ships a `sqlite` feature with `SqlitePrefsStore`. A
  `postgres`/`PostgresPrefsStore` feature exists in the SCOPE
  Crate-layout block by implication; landing the impl is a follow-up
  job. Migrations under `starter_prefs` source name target SQLite
  syntax; the SCOPE "BIGINT on Postgres" note carries forward into
  the follow-up. *Derives from:* SCOPE Crate-layout block explicitly
  lists `starter-store-sqlite`; workspace R5. *Revisit trigger:* a
  consumer ships against Postgres and the sqlite path is no longer
  enough.
- **D-1.3 — `iso_currency` lands on `starter-prefs`, not
  `starter-spi`.** Closes D-U0.3 from Phase 0. The crate uses
  `iso_currency` for two paths: ISO 4217 code validation on
  `PreferencesPatch.currency`, and the locale → currency derivation
  table that powers `currency: "auto"`. *Derives from:* Phase 0
  D-U0.3 ("ISO 4217 validation lives in `starter-prefs`, not
  `starter-spi`"); workspace R5. *Revisit trigger:* a second crate
  needs ISO 4217 validation — at which point `iso_currency` belongs
  in a shared dep (likely `starter-spi` behind a `currency` feature).

**Phase 2 — `Accept-Units` + per-series shape**

- **D-2.1 — Per-series wire shape (R8) lands exactly as written in
  SCOPE §R8.** Canonical form:
  `{ "series": [{ "slot", "quantity", "unit", "points": [[ts_ms, value], …] }] }`.
  Single-value reads: `{ "value", "unit", "quantity" }`. The
  `points` tuple-of-arrays shape is starter-prefs-owned and ships
  as a `ToSchema` DTO in `starter-prefs::dto::series` so the
  OpenAPI doc captures it. *Derives from:* SCOPE R8.
  *Revisit trigger:* a consumer ships timeseries on a different
  shape and the metadata-hoisting principle has to be re-stated.
- **D-2.2 — `UnitsCtx::convert(quantity, value, source_unit)` is
  called at the handler / response-serialiser layer, never as a
  body-rewriting middleware.** `accept_units_layer` inserts
  `UnitsCtx` into request extensions; handlers opt in by calling
  `ctx.convert(...)` on the values they emit. Response bodies are
  not walked or rewritten. *Derives from:* SCOPE R6 (single
  conversion layer per surface; middleware does **not** mutate
  response bodies). *Revisit trigger:* a structured-body rewriter
  ships under a different opt-in (e.g. the Phase 5 diagnostics
  rewriter, which is scope-limited — see D-5.1).

**Phase 3 — `starter-i18n`**

- **D-3.1 — Catalog file format: plain ICU JSON keyed by
  `MessageKey`.** Schema: `{ "<message_key>": "<ICU message
  string>" }`, top-level object only, no nested namespaces in the
  file itself (namespace lives in the key, e.g.
  `"auth.signin.title"`). The loader is `serde_json` with
  `#[serde(deny_unknown_fields)]` on the outer wrapper struct so
  malformed catalogs fail at boot, not at lookup time. *Derives
  from:* SCOPE Library-choices ("plain JSON catalogs, ICU
  MessageFormat — no Fluent in v1"); workspace R7. *Revisit
  trigger:* plural / gender rules outgrow ICU MessageFormat (Fluent
  port) or a consumer needs namespaced sub-objects in the file.
- **D-3.2 — Manifest fingerprint: `sha256` hex, first 16 chars.**
  Computed over the canonicalised catalog JSON bytes (sorted keys,
  no trailing newline). Embedded in the immutable URL:
  `/v1/i18n/catalogs/{lang}-{fingerprint16}.json`. The manifest
  endpoint returns `{ "<lang>": "<fingerprint16>" }`. *Derives
  from:* SCOPE R5 ("content-hash URL for immutable caching");
  workspace R7. *Revisit trigger:* 16 hex chars collide in
  practice (≈ 18 quintillion buckets — not expected) or a stronger
  hash becomes a deployment requirement.

**Phase 4 — `@nube/starter-ui-core`**

- **D-4.1 — `PreferencesProvider` state management: `react-query`
  for the fetch + cache + invalidation seam, `zustand` for ephemeral
  UI state (e.g. the Settings page form draft before save).**
  Matches the existing `@nube/starter-ui-core` posture (workspace
  R6). Query keys namespaced `['starter', 'prefs', ...]`. *Derives
  from:* SCOPE TypeScript-surface block ("React Query + ETag
  caching"); workspace R6. *Revisit trigger:* the workspace migrates
  off react-query (TanStack split, RSC, …) — at which point the
  same seam re-emerges under a new library name.
- **D-4.2 — `IntlProvider` catalog loader is lazy per language,
  manifest-driven.** Boot sequence: fetch `/v1/i18n/manifest` →
  pick the user's resolved `language` from `PreferencesProvider` →
  dynamic `import()` (or `fetch` against the fingerprinted URL) for
  exactly that one language → hand the catalog to `react-intl`'s
  `IntlProvider`. The unresolved English seed bundles ship in-tree
  as a synchronous fallback so first paint never blocks on the
  network. *Derives from:* SCOPE R5 (immutable-cache URL shape);
  D-3.2. *Revisit trigger:* a consumer needs all languages
  pre-loaded (e.g. an offline appliance) — at which point a
  build-time bundling path lands alongside the lazy path.

**Phase 5 — Diagnostics rewriter**

- **D-5.1 — The rewriter touches exactly one envelope shape at
  exactly two top-level paths.** Shape: `{ "diagnostic": { "code":
  "<MessageKey>", "params": { … } } }`. Paths the rewriter looks at:
  (i) the response body's top-level `diagnostic` key, and (ii) the
  top-level `diagnostics` array (each element is the same
  `{code, params}` shape). Nothing else in the body is read,
  walked, or rewritten. SSE / chunked / `text/event-stream`
  responses are bypassed unconditionally — they ship codes + params
  and the consumer translates per event. The rewriter only runs when
  the handler inserts a `DiagnosticBody` response extension; absence
  of the extension is a no-op. Feature-gated on `starter-i18n`'s
  `diagnostics` feature, default-off. *Derives from:* SCOPE R5
  ("scope-limited; declared envelope shape only; documented
  top-level paths; SSE not rewritten"). *Revisit trigger:* a
  long-running-job endpoint emits a diagnostic envelope at a path
  other than these two — at which point the path list grows by PR
  on `starter-i18n` and the change is callout-worthy in the SCOPE.

## Consumer-defined quantities (v2)

The closed-enum design (R4) is correct for v1 — every wire identifier
and every UI label must be known to the platform — but the escape
hatch ("store as dimensionless, format yourself, lose unit prefs") is
genuinely worse than the rest of the system for products that hit
this regularly (AQI, CO₂ ppm, lux, dBA, sound pressure, particulate
counts). The door is open for v2; the shape we'd ship looks like:

- **Namespaced quantity ids on the wire.** Platform quantities stay
  bare (`"temperature"`, `"pressure"`); consumer quantities use a
  namespace (`"com.acme.aqi"`, `"com.acme.co2_ppm"`). The wire
  representation becomes `string` rather than a closed enum; the
  closed enum stays as the *known-platform* subset.
- **Per-deployment registry overlay.** A `register_quantity(spec)`
  call at server boot adds the consumer's quantity + units to the
  `UnitRegistry`. The overlay is per-binary, not per-tenant — every
  user of that deployment sees the same set.
- **UI fallback for unknown quantities.** Studio renders unknown
  namespaced quantities with the unit symbol from the spec and no
  conversion options. Block-defined UI can register richer pickers
  via `registerExtensionMessages` (already in the i18n surface).
- **No cross-deployment portability.** A telemetry row exported from
  deployment A and imported into deployment B that doesn't know
  `com.acme.aqi` renders with the unit symbol but no labels. This is
  the deliberate cost of letting consumers extend the registry.

Not v1. Documented here so consumers evaluating starter for products
in this space know the cost is bounded and the design is not a dead
end.

## Rollout (proposed phases)

Each phase is independently deployable and reversible.

Open work and per-phase open items are tracked in
[TODO.md](./TODO.md). Phase 0's verification report is in
[PHASE0-VERIFY.md](./PHASE0-VERIFY.md).

- **Phase 0 — DONE.** Landed via job
  [`.codeless/jobs/starter-prefs-spi`](../../../.codeless/jobs/starter-prefs-spi/SCOPE.md)
  (stages 1–7, commits `984a262` → `61921d4`). `starter-spi` now
  exposes the `units`, `preferences`, and `i18n` modules with the
  closed `Quantity` / `Unit` enums, `ResolvedPreferences` /
  `PreferencesPatch` DTOs, `LanguageTag` / `MessageKey` /
  `Diagnostic`, and a `StaticRegistry` that routes every conversion
  through `uom`. New starter-spi deps: `uom` (si) and
  `icu_locale_core`. Baseline at
  [`starter-spi-deps.baseline.txt`](./starter-spi-deps.baseline.txt)
  is the CI seam every future starter-spi-touching PR diffs against.
  Headless-appliance smoke (no `starter-prefs` / `starter-i18n` in
  the build graph) holds — those crates do not exist yet by design.
  **Open follow-up:** the `starter-flow-spi` baseline drifted
  (`uom` + `icu_locale_core` transitive through `starter-spi`); see
  [PHASE0-VERIFY.md](./PHASE0-VERIFY.md) §"starter-flow-spi
  baseline — REGRESSION FLAG" and the matching entry in
  [TODO.md](./TODO.md).
- **Phase 1 — TODO.** `starter-prefs` crate: tables + 4 REST
  endpoints + three-layer resolver + `"auto"` derivation +
  `starter-client-rs` methods + a `starter-cli prefs` subcommand.
  End-to-end against a sqlite store. Lands `iso_currency` (D-U0.3:
  ISO 4217 validation lives here, not in `starter-spi`).
- **Phase 2 — TODO.** `Accept-Units` middleware in `starter-server`
  + per-series response shape (R8). Read-path conversion live;
  storage stays canonical. Audit middleware to confirm no log line
  ever sees a converted value (R6).
- **Phase 3 — TODO.** `starter-i18n` crate: catalog loader,
  `Accept-Language` middleware, `GET /v1/i18n/catalogs/{lang}` +
  `/v1/i18n/manifest`, seed `en.json` + `es.json` covering
  starter's own UI strings (auth, errors, settings page chrome).
- **Phase 4 — TODO.** `@nube/starter-ui-core`:
  `PreferencesProvider`, `formatters.ts`, `IntlProvider`,
  `useTranslate`, Settings page wired into `starter-auth-users`'
  account page. Re-export pure formatters + `useTranslate` for
  consumer apps.
- **Phase 5 — TODO.** Diagnostics rewriter (opt-in feature on
  `starter-i18n`) for server-originated messages on long-running
  jobs. Closes the one documented exception to client-side
  translation.

## Smoke tests (before merging)

In addition to the workspace-level smoke tests in [SCOPE.md](../../../SCOPE.md):

### "Headless appliance keeps working" test

A consumer building an edge device compiles a binary with
`starter-auth-token` + `starter-secrets-file` and **without**
`starter-prefs` or `starter-i18n`. The binary builds, runs, and
serves requests as before. No prefs middleware, no extra routes, no
extra migrations. If any of those bleed in, workspace R5 (default-
features minimal) has slipped.

### "Resolver layer precedence" test

Three rows: system default, one org row with `temperature_unit: C,
unit_system: imperial`, one user row with all fields NULL except
`temperature_unit: F`. The resolved view for that user must show
`temperature_unit: F` (user wins for that column) and
`unit_system: imperial` (org wins where user is NULL). Flipping the
user's `temperature_unit` to NULL must then yield `C` (org wins).
Removing the org row entirely must fall through to the hardcoded
system default. This is the single highest-value behaviour in the
crate; it gets a dedicated test.

### "`auto` derivation" test

A user with `locale: en-AU`, `unit_system: metric`, every per-unit
field `"auto"`, and `currency: "auto"` must resolve to: `C`, `kPa`,
`km/h`, `m`, `kg`, `AUD`. Switch `unit_system: imperial` (per-unit
fields still `"auto"`) and the values must flip to `F`, `psi`, `mph`,
`ft`, `lb`. Switch `unit_system` back to `metric` and set
`temperature_unit: F` at the user layer — the result is metric
everywhere except temperature (the BBQ case).

### "Australian operator" test

A user with `timezone: Australia/Brisbane`, `unit_system: metric`,
`temperature_unit: C`, `date_format: DD/MM/YYYY`, `time_format: 24h`
hits `GET /v1/telemetry?slot=temp_in` against a server holding a row
written by a US edge agent (originally `72.4 °F`). The response
carries `"unit": "celsius"`, `"value": 22.44`, the timestamps render
in Brisbane time on the client, and the dates render
`22/04/2026`-style. Switching the user's `unit_system` to imperial
flips the response in-place on next request without a rewrite of the
stored row.

### "MCP raw mode" test

The same request with `Accept-Units: canonical` returns
`"unit": "celsius"`, `"value": 22.44` regardless of the caller's
prefs. No conversion. Stable quantity codes.

### "Add a language" test

Adding `fr.json` to `starter-i18n/catalogs/starter/` and bumping the
crate version is sufficient. No backend deploy required for clients
to use it once they pull the new catalog. Missing keys fall back to
`en` silently.

### "Canonical-only logs" test

Grep every starter crate's log output during integration tests for
the strings `"°F"`, `"psi"`, `"mph"`, `"lb"`. Zero matches. Logs are
always canonical.

### "Custom quantity without forking" test

A consumer wants to add a new quantity (say, `AirQualityIndex`) for
their domain. They cannot add a variant to `Quantity` without a PR
on `starter-spi` — by design. The escape hatch: store the value as
dimensionless (`quantity: None`) and format client-side themselves,
with the tradeoff that users can't set a unit preference for it.
Documented clearly so consumers don't try to monkey-patch the
registry.

## Non-goals

- **Not a translation-management system.** No web UI for editing
  translations, no Lokalise / Crowdin integration, no in-place
  translation editor. Catalogs are JSON files in the repo; consumers
  who want a TMS run their own and emit JSON.
- **Not currency FX.** Money values are stored + displayed in their
  declared currency. Starter never converts between currencies; that
  needs a rate provider, rate caching, and audit logging that are
  out of proportion to the goal of "format a price."
- **Not user-authored content translation.** Translating
  consumer-authored strings (project names, flow descriptions) is a
  separate problem — a sidecar `translations(entity_id, lang, text)`
  table, an authoring affordance in the consumer's UI, and a fallback
  policy. Out of scope for v1; documented as a future extension.
- **Not RTL layout.** Locale parsing knows about script direction;
  layout polish (mirrored chrome, right-aligned text, BiDi-aware
  inputs) is a consumer-app concern.
- **Not accessibility prefs.** Reduced motion, high contrast, font
  scale belong in the same inheritance model and could ship later as
  additional columns; not in the v1 scope.
- **Not per-device timezone sync.** Studio reads the OS timezone and
  may override the profile TZ in `localStorage` for display only.
  The server never sees a per-device TZ — server-originated emails
  use the profile TZ. Splitting at the client is the clean seam.
- **Not a JWT claims spec.** Whether to embed `locale` / `timezone` /
  `language` in tokens is auth-crate territory and depends on the
  authenticator in use; `starter-prefs` always fetches via
  `GET /v1/me/preferences` as the source of truth, with token claims
  (when present) treated as a fast-path hint only.

## Open questions

- **Where do block-defined message keys live?** Probably in a
  registry exposed by `starter-i18n` so consumer apps can call
  `register_catalog(namespace, lang, json)` at startup, with a stable
  precedence (org override → namespace default → platform default).
  Not blocking v1.
- **`Repository<T>` derive for prefs tables?** Deferred to v0.2 with
  the rest of the derive work in `starter-store-*`. Hand-written
  sqlx is fine at this volume.
- **Server-side ICU rendering of money in emails.** `icu_decimal`
  works; whether to also pull `icu_currency` (separate crate) or
  format inline with `iso_currency`'s symbol table. Pick when Phase 5
  lands.

## Bottom line

**Two small optional crates. One trait surface in `starter-spi`. One
resolver, one middleware per concern, canonical-only storage, ICU on
both ends, the same R1–R8 rules as the rest of the workspace.** A new
product gets multi-locale, multi-unit, multi-timezone behaviour by
adding two dependencies and a Settings page — not by retrofitting
i18n into every endpoint a year after launch.
