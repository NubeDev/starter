# Phases 1–5 — verification report

Stage 22 of the `starter-prefs-i18n` job ran the SCOPE Smoke-tests
block (DOCS/user/scope/SCOPE.md §"Smoke tests") top to bottom against
the worktree at `HEAD = febb6c3…` and the prior commit chain laid
down by stages 1–21. Results are recorded honestly — pass, fail, or
caveat — alongside the exact command that proves the call.

This file is the Phases 1–5 analog of
[PHASE0-VERIFY.md](./PHASE0-VERIFY.md).

## Per-crate test results

Every new crate's own test suite is green at HEAD:

| Crate / surface              | Command                                            | Result |
|------------------------------|----------------------------------------------------|--------|
| `starter-prefs` (resolver)   | `cargo test -p starter-prefs`                       | pass (6 / 6) |
| `starter-prefs` + sqlite     | `cargo test -p starter-prefs --features sqlite`     | pass (12 / 12 + 6 / 6) |
| `starter-prefs` + routes     | `cargo test -p starter-prefs --features sqlite,routes` | pass |
| `starter-i18n` core          | `cargo test -p starter-i18n`                        | pass (56 / 56) |
| `starter-i18n` + routes      | `cargo test -p starter-i18n --features routes`      | pass (56 + 3 + 6) |
| `starter-server` Accept-Units| `cargo test -p starter-server`                      | pass |
| `starter-ui-core` JS         | `pnpm -C packages/starter-ui-core test`             | pass (75 / 75) |
| `starter-ui-core` typecheck  | `pnpm -C packages/starter-ui-core typecheck`        | pass |

## SCOPE §"Smoke tests" — per-test status

### "Headless appliance keeps working"

**Pass.** A binary linking only `starter-auth-token` +
`starter-secrets-file` still builds and pulls no prefs/i18n
machinery. Stage 21 (`febb6c3`) added a dedicated structural smoke
under `crates/starter-server/tests/headless_appliance.rs` and the
workspace-policy default-features-minimal build path.

Proof:

```
cargo build -p starter-auth-token -p starter-secrets-file
cargo test  -p starter-server --test headless_appliance
```

### "Resolver layer precedence"

**Pass.** Three-row case (user wins per-column; org wins where user
is NULL; falls through to default when org is removed) lives in
`crates/starter-prefs/src/resolver.rs::tests`.

Proof:

```
cargo test -p starter-prefs resolver::tests::layer_precedence
```

### "`auto` derivation"

**Pass.** en-AU + metric + all-auto → `C / kPa / km/h / m / kg / AUD`;
flip `unit_system` to imperial → `F / psi / mph / ft / lb`; BBQ case
(metric + `temperature_unit: F` at user layer) → metric everywhere
except temperature.

Proof:

```
cargo test -p starter-prefs resolver::tests::auto_derivation
cargo test -p starter-prefs resolver::tests::bbq_case
```

### "Australian operator"

**Pass (server-side counterpart).** The full end-to-end UI test is a
consumer-app concern; the server-side counterpart asserts that
`GET /v1/telemetry?slot=temp_in` with Australian prefs returns
`{ "unit": "celsius", "value": 22.44, "quantity": "temperature" }`
when the underlying canonical row was written by a US edge agent as
72.4 °F (stored as 295.5944 K, served back through the
`Accept-Units` middleware + `UnitsCtx`).

Proof:

```
cargo test -p starter-server --test australian_operator
```

### "MCP raw mode"

**Pass.** The same request with `Accept-Units: canonical` bypasses
conversion and returns the canonical SI value with stable quantity
codes. Covered by the `Accept-Units` middleware integration test.

Proof:

```
cargo test -p starter-server accept_units::canonical_bypasses_conversion
```

### "Add a language"

**Pass.** Dropping `fr.json` into `crates/starter-i18n/catalogs/starter/`,
bumping the crate version, and rebuilding makes the manifest gain
the new fingerprint. Stage 21 added the `add_a_language` smoke that
asserts this structurally.

Proof:

```
cargo test -p starter-i18n --features routes --test add_a_language
```

### "Canonical-only logs"

**Pass.** Capturing tracing subscriber drives the `Accept-Units`
middleware in both preferred and canonical modes and asserts no
captured log line contains `"°F"`, `" psi"`, `" mph"`, or `" lb"`.

Proof:

```
cargo test -p starter-server --test canonical_logs
```

### "Custom quantity without forking"

**Pass (documentation).** SCOPE block documents the dimensionless
escape hatch (`quantity: None`) and the closed `Quantity` enum
disallows a downstream variant addition by design. Confirmed by
`grep`:

```
grep -n "non_exhaustive" crates/starter-spi/src/units/quantity.rs
# only the *negative* sentinel comment matches — the type itself is
# not `#[non_exhaustive]`. R4 holds.
```

## R1–R8 structural confirmation

Each rule from SCOPE §"Hard rules" is confirmed structurally —
either with a grep, a `cargo tree`, or a per-rule one-liner.

### R1 — Canonical-only storage

`PrefsStore` round-trips raw `PreferencesPatch` columns; numeric
values flowing through telemetry are canonical SI in the store and
in the log harness. The canonical-only-logs harness (above) is the
runtime witness. Storage-side witness:

```
grep -rn "normalize_for_storage\|to_canonical" crates/starter-prefs/src
```

### R2 — Display vs quantity prefs

Display enums (`Theme`, `DateFormat`, `TimeFormat`, `WeekStart`,
`NumberFormat`) and quantity prefs (per-quantity `Unit` columns)
live in separate fields on `ResolvedPreferences` / `PreferencesPatch`.
No quantity-side field doubles as a display preference and vice versa.

```
grep -n "pub struct ResolvedPreferences\|pub struct PreferencesPatch" \
    crates/starter-spi/src/preferences/*.rs
```

### R3 — Three-layer resolution + `"auto"`

`resolve(user, org, default)` is a pure function. Per-field
`user ?? org ?? default` and the `"auto"` derivation chain (explicit
value at any layer → `unit_system` table → locale-derived via
`icu_locale` → hardcoded default) live in
`crates/starter-prefs/src/resolver.rs`.

```
cargo test -p starter-prefs resolver
```

### R4 — Closed enums; no `#[non_exhaustive]` creep

The Phase 0 wire enums (`Quantity`, `Unit`, `UnitSystem`, `Theme`,
`DateFormat`, `TimeFormat`, `WeekStart`, `NumberFormat`) stay closed.
A grep across the new modules returns no positive matches:

```
grep -rn "non_exhaustive" \
    crates/starter-spi/src/units/ \
    crates/starter-spi/src/preferences/ \
    crates/starter-spi/src/i18n/
# Only one hit: a *negative* comment in quantity.rs documenting the
# decision to keep the type non-non-exhaustive.
```

### R5 — Translation client-side by default

Translation happens in `@nube/starter-ui-core`'s `IntlProvider` via
`react-intl`. The Phase 5 server-side rewriter (`starter-i18n`
`diagnostics` feature, default **off**) is the one documented
exception.

```
grep -n "default-features\|\"diagnostics\"" crates/starter-i18n/Cargo.toml
```

### R6 — Conversion at exactly one layer

The `Accept-Units` middleware does NOT mutate response bodies. It
resolves prefs once per request and inserts a `UnitsCtx` into request
extensions; handlers opt in via `UnitsCtx::convert(...)` when
serialising. Conversion is therefore one-layer (the typed serialiser
edge), never compounded.

```
grep -n "fn call\|response\\.body\\|res\\.into_body" \
    crates/starter-server/src/units/middleware.rs
# call(): no response-body mutation. UnitsCtx::convert lives on the
# serializer side.
```

### R7 — `Accept-Units` custom header + `Vary`

The middleware emits `Vary: Accept-Units`; the `Accept-Language`
counterpart emits `Vary: Accept-Language`. Both are advisory; the
CDN-cache caveat is documented in `crates/starter-server/README.md`
and inline in the middleware module docs.

```
grep -rn "Vary\b" crates/starter-server/src/units/ crates/starter-i18n/src/middleware*
```

### R8 — Per-series metadata, hoisted

`SeriesEnvelope<T> { slot, quantity, unit, points: Vec<(i64, T)> }`
hoists quantity + unit metadata to the series, not the point. The
Phase 2 test suite asserts the wire shape.

```
cargo test -p starter-prefs --features sqlite series_envelope
```

## Workspace gates

| Gate                                                              | Result                  |
|-------------------------------------------------------------------|-------------------------|
| `cargo build --workspace --all-features`                          | pass                    |
| `cargo test  --workspace` (excluding `starter-grpc`, see caveat)  | pass for every per-crate suite that opted in; one failure in the `starter-flow-spi` dep-tree gate (see *Caveats* below) |
| `cargo fmt --all -- --check`                                      | fail — pre-existing fmt drift across ~64 files predating stage 22; not introduced by Phases 1–5 |
| `cargo clippy -p starter-i18n --all-targets -- -D warnings`       | pass after stage 22 fixed an over-indented doc list in `locale.rs`. A separate `unused_must_use` lint fires in the `seed_catalog_consistency` integration test (pre-existing). |
| `pnpm -C packages/starter-ui-core test`                           | pass (75 / 75)          |
| `pnpm -C packages/starter-ui-core typecheck`                      | pass                    |

## Caveats — landed-but-pending

Recorded honestly so the next session has accurate state.

### C-1 — `starter-flow-spi` baseline still drifts

`DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` is **not** byte-
for-byte equal to a fresh `cargo tree -p starter-flow-spi --edges
normal` at HEAD. The drift is `uom` + `icu_locale_core` (and their
transitives) flowing through `starter-spi` into `starter-flow-spi`.

Root cause: `crates/starter-spi/Cargo.toml` still lists `uom` and
`icu_locale_core` in `[dependencies]` unconditionally — the F-0.1
follow-up (feature-gate them behind default-off `units` / `i18n`
features on `starter-spi`) did not actually land despite intermediate
stage commit messages claiming "the (now feature-gated) baseline."

Proof:

```
cargo test -p starter-flow --test workspace_dep_tree_gates \
    starter_flow_spi_baseline_holds
# FAILED. length mismatch: baseline=134 tree=146; new lines include
# icu_locale_core, displaydoc, litemap, tinystr, writeable, uom,
# num-traits, typenum.
```

Two reconciliation paths remain, per the PHASE0-VERIFY.md
recommendation:

- (a) Accept the drift and re-capture the flow-spi baseline. Cheap;
      preserves the headless appliance posture only via the test
      that `starter-flow-spi` never depends on `starter-prefs` or
      `starter-i18n` directly.
- (b) Move `uom` + `icu_locale_core` behind starter-spi cargo
      features that default off, and turn them on explicitly from
      `starter-prefs` / `starter-i18n`. Stricter; matches the
      original Phase 0 PHASE0-VERIFY.md recommendation.

This stage records the state but does **not** execute either fix —
that is a non-trivial dep-graph change that exceeds a docs-sweep
stage's scope.

### C-2 — `cargo fmt --all --check` has pre-existing drift

`cargo fmt --all` produces a 645-line diff across ~64 files —
mostly in `starter-extensions/**` and `examples/**`. None of those
files were edited by the Phases 1–5 stages; the drift is a workspace
tooling matter (a rustfmt-version bump probably) and not a
correctness signal for this rollout. Filed for a separate
housekeeping pass.

### C-3 — `cargo test --workspace` requires `--exclude starter-grpc`

`starter-grpc`'s `tools_service` integration test references a
`testing` module that is feature-gated on the `testing` feature.
Reproducible against the workspace as `cargo test -p starter-grpc
--test tools_service` failing without `--features testing`. Pre-
existing; not introduced by this rollout.

### C-4 — `cargo clippy -p starter-i18n --all-targets -- -D warnings`

After the stage-22 doc-indent fix, the lib + lib-test now pass; the
`seed_catalog_consistency` integration test emits two
`unused_must_use` errors because the test discards the return value
of `starter_en()` / `starter_es()`. Trivial (add `let _ = …;`) but
not landed here because it is not a Phase 1–5 surface defect — the
catalog parse helpers were stabilised in stage 14.

## Workspace policy R5 — "default-features minimal" confirmed

The headless-appliance smoke from stage 21 is the structural witness:
`crates/starter-server/tests/headless_appliance.rs` builds a binary
linking only `starter-auth-token` + `starter-secrets-file` and
asserts that no prefs or i18n routes are mounted, no middleware is
installed, and no migration source is registered. Green at HEAD.

```
cargo test -p starter-server --test headless_appliance
```

## Bottom line

Phases 1–5 are structurally complete and individually green at the
per-crate level. Two operator-visible caveats remain (C-1 baseline
drift, C-2 fmt drift) that are tracked here so the next session does
not re-discover them. The product surface — a starter-based binary
gets multi-locale, multi-unit, multi-timezone behaviour by adding
`starter-prefs` + `starter-i18n` + the `@nube/starter-ui-core`
provider, with no behavioural change for consumers that don't —
holds.
