# Scope — starter-prefs-spi

> Source of truth: [`DOCS/user/scope/SCOPE.md`](../../../DOCS/user/scope/SCOPE.md)
> §"Rollout (proposed phases)" Phase 0, plus the upstream
> Hard-rules block (especially R1 — store canonical, convert at the
> edge — and R4 — closed `Quantity` / `Unit` enums). This file is
> the per-job brief; intentionally short. When this file disagrees
> with the source-of-truth SCOPE, that doc wins.

## Goal

Land **Phase 0 only**: three new modules inside `starter-spi` —
`preferences`, `units`, `i18n` — exposing the DTOs, enums, trait
seams, and the `StaticRegistry` impl that the rest of the user-
SCOPE phases depend on. No new crates, no storage, no routes, no
middleware. After this job downstream design (Phase 1
`starter-prefs`, Phase 3 `starter-i18n`) can begin without
committing to the wire format twice.

R4 of the user SCOPE applies: `Quantity` and `Unit` are **closed
enums**. Variants added or renamed after this job costs a major
SemVer bump on `starter-spi` plus a deprecation alias. **This is
the one chance to get the enum boundary right.** Stage 1 + the
REVIEW gate exist for that reason.

## In scope

- **`starter-spi::units`** (new module):
  - Closed `pub enum Quantity { Temperature, Pressure, Speed,
    Length, Mass }` (the v1 set the user SCOPE Preferences-model
    column comments enumerate verbatim).
  - Closed `pub enum Unit` with the variants the SCOPE column
    comments enumerate: `celsius`, `fahrenheit`, `kilopascal`,
    `psi`, `bar`, `meter_per_second`, `kilometer_per_hour`,
    `mile_per_hour`, `knot`, `meter`, `foot`, `kilogram`,
    `pound`. Snake_case serde rename to match the wire form the
    SCOPE's "Per-series unit metadata" example uses
    (`"unit": "fahrenheit"`).
  - `pub struct QuantityDef { canonical: Unit, allowed_units:
    &'static [Unit] }`.
  - `pub trait UnitRegistry` with `get(quantity) ->
    Option<&QuantityDef>` and `supports(quantity, unit) -> bool`.
  - `pub struct StaticRegistry` implementing `UnitRegistry` with
    the canonical SI mappings from R1: `Temperature → celsius`,
    `Pressure → kilopascal`, `Speed → meter_per_second`,
    `Length → meter`, `Mass → kilogram`.
  - `pub fn normalize_for_storage(quantity, value, source_unit)
    -> f64` returning the canonical-SI value via `uom`
    internally. Per R4: never hand-written conversion factors.
- **`starter-spi::preferences`** (new module):
  - Closed enums `UnitSystem` (`Metric` / `Imperial`), `Theme`
    (`Light` / `Dark` / `System`), `DateFormat` (`Auto` /
    `IsoYMD` / `DmySlash` / `MdySlash`), `TimeFormat` (`Auto` /
    `H24` / `H12`), `WeekStart` (`Auto` / `Monday` / `Sunday`),
    `NumberFormat` (`Auto` / `CommaDot` / `DotComma` /
    `SpaceComma`).
  - `pub struct ResolvedPreferences` — fully populated; no
    `Option`, no `"auto"` strings (per R3 the resolver collapses
    them before constructing this struct). Fields match the
    SCOPE Preferences-model column list.
  - `pub struct PreferencesPatch` — same fields, all `Option<T>`,
    used by Phase 1's `PATCH /v1/me/preferences` and
    `/v1/orgs/{id}/preferences` endpoints; `None` means "leave
    alone," `Some(None-inside-via-an-explicit-null-sentinel)`
    on the wire means "revert to inherit." The Phase-1 resolver
    owns the inheritance semantics; this crate only ships the
    Patch shape.
  - Serde derives use `#[serde(rename_all = "snake_case")]` on
    every enum so the wire matches the SCOPE column-comment
    strings byte-for-byte. `utoipa::ToSchema` derives per
    workspace R7 (every DTO appears in `openapi.json`).
- **`starter-spi::i18n`** (new module):
  - `pub struct LanguageTag` — BCP-47, validated on construction
    via `icu_locale`. Accepts `en`, `en-US`, `zh-TW`; rejects
    empty / `en_US` (underscore) / non-tags.
  - `pub struct MessageKey` — reverse-DNS-style stable identifier
    for translation lookups (e.g. `flow.error`,
    `auth.token.expired`). Validated newtype; rejects empty,
    whitespace, leading/trailing dots, double dots, non-
    printable characters.
  - `pub struct Diagnostic { code: MessageKey, params:
    BTreeMap<String, DiagnosticParam> }`. `BTreeMap` not
    `HashMap` so the wire form is deterministic — same posture
    `starter-flow-spi::SlotMap` adopts.
  - `pub struct DiagnosticParam` carrying typed interpolation
    values: `String`, `i64`, `f64`, `bool`, and a `Timestamp`
    variant holding UTC epoch ms per R1.
- **Dep landings on `starter-spi`:** `uom` (with the `si`
  feature, default-features = []) and `icu_locale` (no extra
  features beyond default-parsing). Both are explicitly named
  by the user SCOPE Library-choices block; Phase 0 is where
  they enter the dep tree.
- **Baseline file:** `DOCS/user/scope/starter-spi-deps.baseline.txt`
  — the analog of `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
  the merged `starter-flow-scaffold` sibling landed. CI gate
  for Phase 1 (and every future starter-spi-touching PR) is
  this file matching `cargo tree -p starter-spi --edges normal`
  byte-for-byte.
- **Module wiring + tests** in the same commits per the
  workspace's "tests live with the code" rule.

## Out of scope

- **`starter-prefs` and `starter-i18n` crates.** Phase 1 + Phase 3
  respectively. The two crates do not exist yet; this job only
  lands the shared wire types in `starter-spi`.
- **Storage.** No `PrefsStore` trait, no migrations, no sqlite
  or postgres impls. Phase 1.
- **REST routes.** No axum routers, no
  `GET /v1/me/preferences`, no `GET /v1/units`. Phase 1.
- **Middleware.** `accept_units_layer` and `accept_language_layer`
  are Phase 2 + Phase 3 respectively. This crate does not depend
  on `axum` or `tower` and the cargo-tree gate enforces that.
- **Resolver.** The three-layer `user → org → default` resolver
  is the Phase 1 `starter-prefs::resolve` function. Phase 0
  ships the `ResolvedPreferences` struct it returns; not the
  function.
- **`auto` derivation.** The `"auto"` placeholder semantics from
  R3 live in the resolver, not in the DTOs. The DTO fields are
  typed enums; `"auto"` lives only on the `PreferencesPatch` side
  as `None`. Phase 1 owns the derivation.
- **Catalogs / translations.** `en.json`, `es.json`, the
  catalog loader, the bundle, the `Accept-Language` parser are
  Phase 3 `starter-i18n` work. This crate ships `LanguageTag` and
  `Diagnostic`; not the catalog format.
- **TypeScript / UI work.** Phase 4. Out of scope here.
- **Diagnostics rewriter.** Phase 5.
- **Currency FX, RTL layout, accessibility prefs, JWT claims
  spec.** Listed Non-goals in the user SCOPE; still out.
- **Adding new `Quantity` or `Unit` variants beyond the v1
  list.** Friction is intentional per R4. AQI / CO₂ ppm / lux
  cases land via the v2 namespaced-quantities path the SCOPE
  documents; not here.
- **`Ratio` quantity.** Documented in the SCOPE Decisions-made
  block as "canonical is 0.0..=1.0; Percent is display-only"
  but no v1 preferences-model field needs it. Deferred until
  a consumer surfaces a need; Phase 0 leaves it off `Quantity`.

## Hard rules (load-bearing — inherited from user SCOPE)

Restated so the runner re-reads them every stage:

- **R1 — Store canonical, convert at the edge.** Phase 0 DTOs
  carry typed canonical units. `Timestamp` is UTC epoch
  milliseconds (`i64`). Money would be minor-units `i64` + ISO
  4217 string; Phase 0 ships the `Unit` enum without any money
  variants because money is its own type per R1.
- **R3 — Three-layer resolution: user → org → default.** Phase
  0 ships the DTO shapes the resolver returns; the resolver
  itself is Phase 1. `ResolvedPreferences` has no `Option` and
  no `"auto"` so callers downstream of the resolver never
  handle NULL semantics.
- **R4 — One source of truth for the unit registry; closed
  enums.** `Quantity` and `Unit` are closed. Adding a variant
  later is backward-compatible; renaming or removing requires a
  major SemVer bump + deprecation alias for at least one major.
  Stage 1 + the REVIEW gate exist because this rule makes the
  Phase-0 enum boundary load-bearing for the workspace.
  Conversion factors are delegated to `uom`; never hand-written.
- **R8 — Per-series unit metadata, not per-value.** Phase 0
  doesn't ship a timeseries shape (Phase 1 / consumer-side
  responsibility), but the snake_case serde rename on `Unit`
  matches the SCOPE example
  (`"unit": "fahrenheit"`) so the future timeseries surface
  has nothing to retrofit.

## Constraints

- **`starter-spi` deps stay minimal.** Adds `uom` (si feature)
  and `icu_locale` (default features). Does NOT add `axum`,
  `tower`, `tokio` (already pulled by other modules), `chrono`,
  or `time`. The cargo-tree baseline in stage 6 enforces this.
- **`default-features = []` posture stays on `starter-spi`.**
  Existing posture; Phase 0 does not flip any defaults.
- **R7 (workspace): every public DTO derives
  `utoipa::ToSchema`.** Every type added in stages 4 and 5
  must derive `ToSchema` so the auto-generated `openapi.json`
  picks them up automatically when Phase 1's routes mount.
- **Snake_case wire renames** on every enum, matching the SCOPE
  column-comment strings byte-for-byte. Stage 4 tests
  round-trip JSON against the SCOPE strings.
- **MSRV 1.78** (workspace). `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo fmt --check`
  non-negotiable.
- **No `unsafe`.** Same forbid posture as existing
  `starter-spi` modules.
- **Tests live with the code.** Each stage commits its tests in
  the same commit as the bodies.
- **`starter-flow-spi` baseline unchanged.** Phase 0 must not
  cause `cargo tree -p starter-flow-spi --edges normal` to
  drift — `starter-flow-spi` depends on `starter-spi` but only
  on the existing modules, not the new ones. The stage-7 diff
  against `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
  catches this.

## Decisions

Locked in stage 1. Each decision lists the rule it derives from
and the **revisit trigger** — the event that should reopen the
question. Anything else is noise.

### D-U0.1 — `Quantity` v1 variants

`Quantity` ships with exactly five variants: `Temperature`,
`Pressure`, `Speed`, `Length`, `Mass`. **`Ratio` is NOT in v1.**
Money is not a `Quantity` (it has its own `i64 minor-units + ISO
4217 string` shape per R1).

- **Why.** Matches the per-unit columns enumerated in the user
  SCOPE Preferences-model block (`temperature_unit`,
  `pressure_unit`, `speed_unit`, `length_unit`, `mass_unit`).
  These are the five quantities the preference model actually
  has fields for; adding more without a corresponding
  preference field is YAGNI.
- **Revisit when.** A new preference column lands (e.g.
  `area_unit`, `volume_unit`) — at that point the corresponding
  `Quantity` variant + `Unit` variants land in the same PR per
  R4 versioning rules. Or: a consumer hits the v2
  namespaced-quantities path documented in SCOPE §"Consumer-
  defined quantities (v2)" and we open that door.

### D-U0.2 — `Unit` v1 variants

`Unit` ships with exactly the variants the SCOPE Preferences-
model column comments enumerate: `celsius`, `fahrenheit`,
`kilopascal`, `psi`, `bar`, `meter_per_second`,
`kilometer_per_hour`, `mile_per_hour`, `knot`, `meter`, `foot`,
`kilogram`, `pound`. Snake_case serde renames match the SCOPE
"Per-series unit metadata" example byte-for-byte.

- **Why.** Closed-enum guarantee under R4. Variants beyond what
  the v1 preferences columns reference are speculative; YAGNI.
- **Revisit when.** A consumer needs a unit not in this list
  (e.g. `inch`, `mile`, `tonne`) for the workspace's own
  features. Land via PR + major bump cycle per R4 versioning.

### D-U0.3 — `currency` wire form

`currency` is `String` (ISO 4217 code), not an enum. `iso_currency`
crate is NOT pulled into `starter-spi` at Phase 0; ISO 4217
validation lives in Phase 1's resolver where the table is
actually consulted.

- **Why.** Adding `iso_currency` to `starter-spi` would pull a
  static currency table into every dependent crate. The Phase 0
  shape is the wire type only; validation is the resolver's
  job, same as locale string validation against ICU.
- **Revisit when.** A second crate needs to validate ISO 4217
  before Phase 1 lands. Then `iso_currency` moves into
  `starter-spi`; until then it stays in `starter-prefs` (Phase 1).

### D-U0.4 — `Diagnostic` param map type

`Diagnostic::params` is `BTreeMap<String, DiagnosticParam>`,
not `HashMap`. Deterministic iteration on the wire.

- **Why.** Matches the existing `starter-flow-spi::SlotMap`
  posture (`BTreeMap<String, SlotValue>`). Snapshots, diffs,
  and the i18n translation cache key behave predictably.
- **Revisit when.** A profile shows BTreeMap insertion is a
  hotspot in a real production flow (very unlikely at the
  per-diagnostic cardinality this carries). At that point
  switch to `IndexMap` or a stable-iteration `HashMap`.

## Cross-cutting checks the runner must keep honest

- **Closed-enum invariant** — `grep -rn '#\[non_exhaustive\]'
  crates/starter-spi/src/units` and `crates/starter-spi/src/preferences`
  return zero hits on the v1 enums (`Quantity`, `Unit`,
  `UnitSystem`, `Theme`, `DateFormat`, `TimeFormat`, `WeekStart`,
  `NumberFormat`). R4 says closed; `#[non_exhaustive]` would
  silently weaken the guarantee.
- **No middleware deps in `starter-spi`** — `cargo tree -p
  starter-spi --edges normal | grep -E '^.* (axum|tower|hyper)'`
  returns empty. Same for `chrono` and `time` — `jiff` is the
  workspace's chosen datetime crate per SCOPE Library-choices,
  but Phase 0 doesn't need datetime; deferred to Phase 1.
- **`uom` lands once** — `cargo tree -p starter-spi --edges
  normal | grep -c '^.* uom '` returns 1 (top-level), and no
  other crate gains an unintended `uom` dep through transitive
  inclusion this phase.
- **Baseline matches** — `cargo tree -p starter-spi --edges
  normal | diff - DOCS/user/scope/starter-spi-deps.baseline.txt`
  is empty at stage-7 end.
- **`starter-flow-spi` baseline unchanged** — same `diff`
  against `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
  is empty. Phase 0 must not regress the merged
  `starter-flow-scaffold` sibling's baseline.
- **Snake_case wire** — JSON round-trip test in stage 4 confirms
  serialised enum variants match the SCOPE column-comment
  strings (`"metric"`, `"light"`, `"monday"`, etc.) without
  any per-field rename overrides.

## Deliverables

- `crates/starter-spi/src/units/` (or `units.rs`) populated.
- `crates/starter-spi/src/preferences/` (or `preferences.rs`)
  populated.
- `crates/starter-spi/src/i18n/` (or `i18n.rs`) populated.
- `crates/starter-spi/src/lib.rs` declares the three new
  modules in alphabetical order.
- `crates/starter-spi/Cargo.toml` adds `uom` (`si` feature) and
  `icu_locale` (default-parsing) under `[dependencies]`.
- `DOCS/user/scope/starter-spi-deps.baseline.txt` committed,
  matches the post-Phase-0 cargo-tree output.
- Unit tests in each new module landing alongside.
- `cargo build --workspace --all-features` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo fmt --check` green.
- `cargo tree -p starter-flow-spi --edges normal` matches the
  existing `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
  byte-for-byte (Phase 0 must not regress Phase-1-of-flow).

## Open questions (resolve in stage 1)

1. **Module file shape.** `units/mod.rs` + `units/registry.rs` +
   `units/normalize.rs` (directory module per workspace ≤ 400
   line/file rule), or single-file `units.rs` if total lines stay
   under the ceiling? Bias: directory module, matches existing
   `starter-spi/src/ai/` and `auth/` precedent.
2. **`StaticRegistry` shape.** `&'static` slices + a `OnceLock`
   constructor, or a built-on-first-call lazy via `OnceCell`?
   Bias: `&'static` slices with a function returning a static
   reference — no allocation, no synchronisation overhead, the
   v1 set is small enough.
3. **`DiagnosticParam::Timestamp` carrier type.** `i64` (epoch
   ms) plain, or a newtype `Timestamp(i64)` to make the unit
   explicit at the type level? Bias: newtype `Timestamp(i64)`
   with `From<i64>` so callers don't have to remember; matches
   the spirit of R1 ("never a TZ-offset sidecar — timezone
   belongs on the user").
4. **`Unit` Display impl format.** The snake_case wire form
   (`"meter_per_second"`) or a human-friendly form (`"m/s"`)?
   Bias: two impls — `serde` uses `snake_case` for the wire
   (locked in D-U0.2); `Display` uses the human-friendly form
   (`"m/s"`, `"°C"`, `"kPa"`) for CLI / log output. Sub-decision
   recorded under D-U0.2's "Naming convention" once locked.

D-U0.1 through D-U0.4 above are locked in stage 1; the four
above are sub-decisions whose outcome lands under "Decisions"
once stage 1 finishes.
