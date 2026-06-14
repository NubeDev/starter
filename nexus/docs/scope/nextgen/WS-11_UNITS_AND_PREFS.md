# WS-11 — Units & Datetime Preferences (backend-side conversion)

> **Status:** Proposal · **Wave:** 1 (server convert path) + 2 (panel/kind quantity tagging) · **Owner:** _unassigned_
> **Depends on:** `starter-prefs` + `starter-spi/units` (exist; some stages incomplete) · couples with WS-04, WS-03/10, WS-09
> **Migration:** block `15xx` (e.g. `1501_prefs.sql` — port `starter-prefs` PG schema under RLS) · **Read first:** GAP_ANALYSIS §2.11, ROADMAP §0
> **Verified:** `nexus-gaps` on 2026-06-09 — corrected: `Accept-Units`/`UnitsCtx` is **already built** in `starter-server` (§2), AND the `starter-prefs` **Postgres store is NOT deferred** — `PgPrefsStore` (`store.rs`, `#[cfg(feature = "postgres")]`) + `migrations/postgres/0001_starter_prefs.sql` + `tests/postgres_store.rs` all ship. WS-11 reuses the PG store as-is; the real gap is the nexus tenancy wiring (route-pinned isolation) + mounting `Accept-Units`. Re-grep before building (ROADMAP §0).
>
> **Why this exists:** the user wants **values converted to the user's preferred units + date/time
> format on the *backend*, not the client** — so every consumer (the web UI, a future mobile app,
> alert notifications, CSV/PDF exports, the API itself) gets correctly-converted output for free,
> with one implementation. The platform **already has the machinery** (`starter-spi/units` +
> `starter-prefs`); nexus just hasn't wired it in. This is mostly *integration*, not green-field.
>
> 🚧 **KNOWN BLOCKER (cross-repo, on the critical path for the domain) — closed `Quantity` enum.**
> `starter-spi`'s `Quantity` is **closed** (only the platform can add variants) and today has:
> Temperature, Pressure, Speed, Length, Mass, Duration, Volume, Energy, Power, Area, Angle, Frequency
> (verified 2026-06-09, `units/quantity.rs`). For an **energy/water/HVAC** product that is missing
> real domain quantities — most glaringly **volumetric flow-rate** (water/air: m³/s, L/min, GPM,
> CFM), plus likely **mass-flow, electrical current/voltage, humidity, and concentration (ppm)**.
> A nexus session **cannot add these locally** — it's an **upstream `starter-spi` change** (new enum
> variants + `uom` arms in `convert.rs` + registry entries), affecting every app on the crate. **This
> is a real cross-repo dependency on the critical path, not a footnote.** Action: as part of Wave 0,
> inventory the domain quantities the first dashboards actually need and **raise the `starter-spi`
> additions early** so they land before WS-11 tags series. Until then, those measures fall back to
> bare-number passthrough (no conversion) — acceptable as an interim, but call it out per affected panel.

---

## 1. The idea in one paragraph

Store every numeric measurement in nexus in its **canonical SI unit** (°C, m/s, kWh→J, …) and store
every timestamp as **UTC**. At the **response edge**, the server resolves the caller's preferences
(user → org → system default), converts each value to the preferred unit, and tags the series with
`{quantity, unit}` once per series (not per point). Date/time formatting context (timezone, date
format, 24h/12h, week-start, number format) ships alongside so any client renders consistently
without its own conversion logic. The conversion is **opt-in per response** via an `Accept-Units`
header (`preferred` | `canonical`), so power users / exports can ask for raw canonical numbers. This
is the **"convert at the presentation edge, never in storage"** rule the starter platform already
mandates (SCOPE R1) — WS-11 makes nexus obey it.

---

## 2. What already exists (don't rebuild — this is the win)

Read these before writing anything; the design is done, much of the code is too.

### `starter-spi/src/units/` — the conversion core (complete)
- **`Quantity`** (closed enum): Temperature, Pressure, Speed, Length, Mass, Duration, Volume,
  Energy, Power, Area, Angle, Frequency. **`Unit`** (closed enum): every wire code
  (`celsius`/`fahrenheit`, `meter_per_second`/`mile_per_hour`/`knot`, `kilowatt_hour`, …).
  *Closed by design (SCOPE R4) — extensions can't add variants; every wire id is platform-known.*
- **`normalize_for_storage(quantity, value, source_unit) -> canonical`** and
  **`from_canonical(quantity, canonical, target_unit) -> value`** — conversion math delegated to
  `uom`; this is the *only* place that names `uom`. (`units/convert.rs`)
- **`convert_for_display(quantity, canonical, target_unit) -> Converted { original, value, unit,
  symbol }`** — one-shot presentation helper with the display symbol (`°C`, `kWh`). (`units/display.rs`)
- `UnitMetadata`, `StaticRegistry`/`UnitRegistry` (canonical lookup per quantity), `GET /v1/units`.

### `starter-prefs/` — three-layer preference resolution
- **`resolve(user, org, default) -> ResolvedPreferences`** — pure function, user → org → system
  default *per column*, with `"auto"` derivation (e.g. `unit_system: imperial` → Fahrenheit/mph;
  `currency: auto` from locale). Fully concrete output, no `Option`/`"auto"` left. (`resolver.rs`)
- `ResolvedPreferences` carries: `timezone`, `locale`, `language`, `unit_system`, per-quantity units
  (`temperature_unit`, `pressure_unit`, `speed_unit`, `length_unit`, `mass_unit`), and display fields
  `date_format`, `time_format`, `week_start`, `number_format`, `currency`, `theme`.
- `PrefsStore` trait (**both sqlite AND Postgres done** — `PgPrefsStore` ships behind the `postgres`
  feature, contrary to the original "deferred" note; see §5), REST `GET/PATCH /v1/me/preferences`
  + `/v1/orgs/{id}/preferences`, prefs-resolution **middleware** (stashes resolved prefs in request
  extensions). `money.rs` (minor-units + ISO 4217).

### `SeriesEnvelope<T>` — the wire shape (complete, in `starter-prefs/src/dto/series.rs`)
The R8-mandated per-series shape — `{quantity, unit}` hoisted once per series, `points: [[ts,
value]]`:
```json
{ "series": [{ "slot": "temp_in", "quantity": "temperature", "unit": "fahrenheit",
               "points": [[1713456000000, 72.4], [1713456060000, 72.6]] }] }
```
Plus the adapter traits `ToCanonicalSeries` / `FromCanonicalSeries` for typed structs.

### `Accept-Units` middleware + `UnitsCtx` — **already built, in `starter-server`** (corrected)
> **Verified (2026-06-09):** the convert path is **not** an unbuilt "Phase 2 scaffold" — it is
> implemented and integration-tested. `crates/starter-server/src/middleware/accept_units.rs` is
> **~616 lines** with tests `crates/starter-server/tests/{australian_operator,canonical_logs}.rs`.
> `UnitsCtx` lives there + in `starter-server/src/middleware/mod.rs`. The empty scaffold comment I
> previously quoted is in `starter-prefs/src/middleware.rs` (the *prefs-resolution* tower layer) —
> a **different** file from the *units-conversion* layer, which is done. **WS-11 consumes this; it
> does not build it.** The remaining unknown is whether the prefs-resolution middleware in
> `starter-prefs` is wired end-to-end on PG (see below) — re-grep before assuming.

### What is NOT yet built (the real, narrower integration gap WS-11 fills)
- **`starter-prefs` Postgres store** EXISTS (`PgPrefsStore`, tested) — the original "deferred
  (sqlite-only)" note was stale. The real gap is **nexus tenancy wiring**: port
  `starter-prefs/migrations/postgres/0001_starter_prefs.sql` → a nexus migration granted to the
  runtime role, then enforce cross-tenant isolation. The store runs outside `tenant_tx` (no
  `app.tenant_id` GUC), so isolation is **route-pinned** (`workspace_id = principal.tenant_id`), not
  RLS-bound — nexus owns thin `/api/v1/me/preferences` routes rather than mounting the starter
  router's spoofable `?org=` selector.
- **Mount the (existing) `Accept-Units`/`UnitsCtx` layer** from `starter-server` in nexus-api and
  apply it at the query/stream response edge. This is *wiring an existing middleware in*, not writing
  it. If a nexus-specific gap surfaces (e.g. SSE-path conversion), scope only that delta.
- **nexus knows nothing about quantities yet.** Panels/queries return bare numbers; nothing declares
  "this column is a temperature." WS-11's real nexus-side work is *tagging series with a quantity* so
  the envelope + conversion can run. **This is the bulk of the actual effort.**

---

## 3. The design for nexus

### 3.1 The data contract: a panel/series declares its quantity
Conversion needs to know *what a column is*. Extend the field-mapping model (couples with **WS-04**
panel editor and the **WS-10** kind `_params`/output declaration) so a series can carry:
```ts
SeriesField {
  value: string;               // result column (existing)
  quantity?: Quantity;         // NEW: "temperature" | "power" | ... (enables conversion)
  storedUnit?: Unit;           // NEW: the unit the DB column is stored in (default: canonical)
  // unit/label/color stay; `unit` becomes a *display override*, else from prefs
}
```
- If `quantity` is set, the server can convert. If absent, the value passes through untouched
  (back-compat — bare numbers still work).
- `storedUnit` lets us adopt data that *isn't* canonical (a legacy table storing °F) without a
  migration: normalize on read. New nexus-owned tables should store canonical.

### 3.2 The conversion path (server-side, opt-in)
1. **Resolve prefs once per request** via the `starter-prefs` middleware → `ResolvedPreferences` in
   request extensions (keyed off `Principal`'s user + org/tenant).
2. **Build/lift the result into `SeriesEnvelope<f64>`** — the query handler maps each declared
   `quantity` column into a series, in **canonical** units (normalizing via `storedUnit` if needed).
3. **Convert at the edge** with a `UnitsCtx` built from the resolved prefs: for each series, pick the
   preferred `Unit` for its `Quantity` (`ResolvedPreferences.<quantity>_unit`) and run
   `from_canonical`. Tag the envelope `unit` = preferred unit.
4. **`Accept-Units` header** decides: `preferred` (default — convert) vs `canonical` (raw SI, for
   exports / debugging / a client that wants to convert itself). This middleware **already exists** in
   `starter-server` (`accept_units.rs`); WS-11 **mounts it in nexus-api**, not reimplements it.

### 3.3 Datetime: store UTC, ship formatting context — **the UI seam already exists**
- All timestamps stored/queried as UTC (already the norm; enforce it).
- The response (or a `GET /api/v1/me` extension) ships the resolved `{timezone, date_format,
  time_format, week_start, number_format, locale}` so the client formats consistently. **The actual
  string formatting stays client-side** (locale/ICU-heavy, cheap on the client) — but the *policy*
  (which tz, which format) is backend-resolved, so a future app obeys the same prefs without
  re-deriving them.
- **The nexus UI is already pre-wired for this — don't rebuild it, *feed* it.** `ui/src/datetime/`:
  - `useDateTime.ts` already resolves **`PreferencesContext` (org/user prefs) first**, falling back
    to the local `datetime/store.ts` settings — its own comment calls this *"the plug in org/user
    prefs later seam."* WS-11's UI job is to **mount `<PreferencesProvider>` populated from the nexus
    `/me` resolved prefs** so the first branch fires. Every `useDateTime()` call site then formats
    via backend-resolved prefs with **zero call-site changes**.
  - `datetime/store.ts` `DateTimeSettings` is documented as mirroring `ResolvedPreferences`
    date/time fields **1:1** — so the mapping from the backend `ResolvedPreferences` onto the
    provider is mechanical. The local store stays as the no-backend / per-device fallback.
  - `datetime/regions.ts` region presets remain a convenience quick-set; backend prefs override.
- **Optional stronger form:** for exports/notifications where there's no smart client, the server
  emits fully-formatted strings too. Do this for alert messages + CSV/PDF (see §3.5).

### 3.4 Where conversion applies (the surfaces that benefit — the whole point)
The reason to do this backend-side: **one implementation, every consumer.**
- **Panels** (`POST /query` / kinds) — series converted per viewer's prefs.
- **Live panels (SSE)** — same conversion on each streamed batch.
- **Alert evaluation + notifications** — *evaluate* on canonical values (thresholds are unit-stable),
  but *render* the notification in the recipient's preferred unit + tz ("Tank 3 at 95 °F" not "35 °C"
  to a US operator). Couples with **WS-07**.
- **Exports** (future CSV/PDF) — converted + formatted server-side.
- **A future mobile/native app** — calls the same API, gets converted values, zero client unit code.

### 3.5 Threshold/min/max are unit-aware too (couples with WS-04)
A gauge threshold "warn at 80" is meaningless without a unit. Store thresholds in **canonical** and
convert for display, OR store with an explicit unit and normalize. Pick canonical-storage to match
everything else. The panel editor (WS-04) shows thresholds in the viewer's preferred unit.

### 3.6 Widget integration — the concrete seam (the user's "must work with the widgets" ask)
The widget layer (`ui/src/features/widgets/`) already separates *data subscription* from *pure
render*, which is exactly where units + datetime plug in. Three precise touchpoints:

1. **`useWidgetQuery.ts` — request + cache key.** Today it sends `{ sql }` and keys on
   `["nexus","query", datasourceId, sql]`. WS-11 adds the **`Accept-Units` header** (preferred vs
   canonical) and **folds the resolved units/locale into the query key** so two users with different
   prefs don't share a cache entry (mirror the WS-09 server cache; the client key must agree). Once
   conversion is server-side, the widget receives **already-converted values + the series `unit`**.
2. **Value consumers read the returned unit, don't convert.** `scalar.ts` (`latestValue` /
   `previousValue`), `statDelta.ts`, `thresholdState.ts`, and the per-type option builders
   (`gaugeOption.ts`, `lineOption.ts`, `barOption.ts`, `pieOption.ts`, `scatterOption.ts`,
   `heatmapOption.ts`) currently read raw `field.value` numbers and a free-text `unit` label. WS-11
   feeds them the **server-converted value** and the **server-returned `unit`/`symbol`** (from the
   `Converted`/`SeriesEnvelope` shape) for axis labels, tooltips, stat suffixes, and gauge ranges.
   Thresholds (`thresholdState.ts`) compare in the **same (converted) space** as the displayed value
   — or, cleaner, compare canonical server-side and ship the state. Decide one (recommend: display
   converted, evaluate canonical, consistent with alerting §3.4).
3. **Datetime axis/cells via `useDateTime()`.** Time axes and `kind:"time"` table columns already
   route through `useDateTime()` (see `lineOption.ts`, `DeviceTable.tsx`). Once §3.3 mounts the
   `PreferencesProvider`, those render in the user's tz/format automatically — **no widget changes**.

Net: the widgets become **unit/format-agnostic** — they render whatever converted value + unit label
the server hands them. Adding a new viz type later inherits correct units/dates for free.

---

## 4. Scope (this workstream)

1. **Stand up `starter-prefs` on Postgres for nexus**: port the prefs schema migration under the
   runtime/RLS role; implement/enable the PG `PrefsStore`; mount the prefs middleware in nexus-api.
2. **Mount the existing `Accept-Units` + `UnitsCtx` middleware** (`starter-server/.../accept_units.rs`,
   already built + tested) in nexus-api and apply it at the query/stream response edge. Build only any
   genuine nexus-specific delta (e.g. SSE-path conversion), not the middleware itself.
3. **Extend nexus DTOs**: add `quantity` + `storedUnit` to the series/field-mapping model
   (`nexus-spi` + `ui/src/data/types.ts`); emit `SeriesEnvelope` from the query handler when a
   quantity is declared; keep bare-number passthrough for untagged series.
4. **Resolve + expose prefs to the UI**: extend `GET /api/v1/me` (or a `GET /api/v1/me/preferences`)
   to return `ResolvedPreferences`; a preferences screen (units + datetime + locale) writing
   `PATCH /me/preferences`.
5. **Wire the UI** (`ui/src/datetime/` + `ui/src/features/widgets/`, see §3.3 + §3.6):
   - Mount `<PreferencesProvider>` fed by `/me` resolved prefs so `useDateTime.ts`'s existing
     org/user-prefs branch activates (no call-site changes).
   - Thread `Accept-Units` + units/locale into `useWidgetQuery.ts` (header + query key).
   - Have value consumers (`scalar.ts`, `statDelta.ts`, `thresholdState.ts`, the `*Option.ts`
     builders) label with the server-returned `unit`/`symbol` instead of a free-text unit string.
   - Prefs screen: units + datetime + locale, writing `PATCH /me/preferences`; the existing
     `datetime/store.ts` becomes the no-backend per-device fallback only.
6. **Alerting (with WS-07)**: evaluate canonical, render notifications in recipient prefs.
7. **Thresholds/min/max canonical-storage + display-convert (with WS-04).**

## 5. Open questions to settle in Wave 0
1. **Prefs ownership**: does nexus reuse `starter-prefs` tables directly (one prefs store across the
   platform) or keep its own org/user prefs linked to nexus tenants? Recommend **reuse starter-prefs**
   (it's the platform contract) + a thin link to nexus `tenant_id` as the "org" layer. `0015_*` only
   if a link table is needed.
2. **`Accept-Units`/`UnitsCtx` is already in `starter-server`** (verified) — so the future app gets it
   free, which is the user's exact motivation. The only question is whether the nexus integration
   surfaces a gap worth upstreaming (e.g. SSE conversion); if so, upstream it into `starter-server`,
   don't fork it into nexus.
3. **Quantity tagging source**: manual per-panel (WS-04), or can a **WS-10 kind** declare its output
   columns' quantities in the manifest (so a kind is self-describing and conversion is automatic)?
   Recommend kinds declare output quantities → conversion "just works" for kind-backed panels.
4. **Formatting strings server-side**: client-formats for the web app; server-formats for
   notifications/exports. Confirm the split (§3.3).

## 6. Acceptance criteria
- [ ] A panel series tagged `quantity: temperature` returns Fahrenheit to a user whose prefs resolve
  to imperial, Celsius to a metric user — **same stored canonical value**, converted server-side.
- [ ] `Accept-Units: canonical` returns raw SI; default returns preferred.
- [ ] Prefs resolve user → org(tenant) → default; `unit_system: imperial` flips all unsplit-by-system
  quantities; explicit per-unit override wins.
- [ ] Timestamps are UTC in storage; the response carries the resolved tz/date/time-format context.
- [ ] An alert notification renders value + time in the *recipient's* prefs; evaluation uses canonical.
- [ ] Untagged series still return bare numbers (back-compat).
- [ ] Prefs persist on Postgres under the RLS role; cross-tenant isolation holds.
- [ ] **Widgets reflect it:** a gauge/stat/line panel shows the converted value + the server-returned
  unit symbol; thresholds and axis labels are in the viewer's unit; time axes/cells render in the
  viewer's tz/format via the existing `useDateTime()` once `PreferencesProvider` is mounted — with no
  per-widget conversion code.
- [ ] `useWidgetQuery.ts` query key includes resolved units/locale (two users with different prefs do
  not share a cached result).
- [ ] Tests: convert round-trip (already in starter — add nexus-edge tests), pref resolution at the
  nexus edge, `Accept-Units` switch, envelope emission, alert-notification rendering, and a widget
  test asserting the rendered unit/symbol matches resolved prefs.

## 7. Out of scope (hand off / defer)
- New `Quantity`/`Unit` variants — closed enums (SCOPE R4), so this is an **upstream `starter-spi`
  change**, not a nexus change. **Promoted from a footnote to the header BLOCKER above** (flow-rate
  etc. for water/HVAC). Out of scope to *implement here*, but **in scope to raise upstream in Wave 0**
  — it's a critical-path cross-repo dependency, not a someday-maybe.
- FX/currency conversion — `starter-prefs/money.rs` is store-only, no FX (explicit non-goal).
- ICU-driven locale-default derivation for date/number formats — starter defers it; nexus inherits
  whatever starter ships.

## 8. Relationship to other workstreams
- **WS-04 (panel editor):** the unit picker becomes "pick a quantity → preferred unit is automatic
  from prefs, with an optional display override." Thresholds become unit-aware. Tightly coupled.
- **WS-03 / WS-10 (kinds):** a kind can declare its output columns' quantities → kind-backed panels
  convert automatically (no per-panel tagging). Strong reason to land kinds + units together.
- **WS-07 (alerting):** evaluate canonical, notify in recipient prefs — needs WS-11's convert path.
- **WS-09 (caching):** the **result-cache key must include the resolved units/locale** (or two users
  with different prefs share a cache entry and see wrong units). Mirror the rubix two-layer cache:
  cache the **canonical** query result at *tenant* scope, the **converted/rendered** output at *user*
  scope — one DB hit serves every user, per-user conversion paid once per TTL. Feed this into WS-09 §P1.
- **WS-01 (time range):** `now`/relative ranges resolve in the user's timezone (from prefs).
