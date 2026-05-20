## Done

- Implemented `pub fn resolve(user, org, default) -> ResolvedPreferences` in `crates/starter-prefs/src/resolver.rs` as a pure, deterministic function with no I/O.
- Defined `UserPrefsRow`, `OrgPrefsRow`, `SystemDefaults`, plus `UnitPref { Auto, Explicit(Unit) }` and `StringPref { Auto, Explicit(String) }` storage-shape types.
- R3 per-field precedence (user → org → default) plus the per-unit "auto" derivation order via the metric/imperial table; currency "auto" derives from locale via `iso_currency`.
- 7 unit tests in `crates/starter-prefs/src/resolver/tests.rs` covering the three SCOPE Smoke-tests-block cases (layer precedence, en-AU metric auto-all → C/kPa/km/h/m/kg/AUD, imperial flip → F/psi/mph/ft/lb, BBQ override). `cargo test -p starter-prefs` green; clippy + fmt green.
- Committed as `7680361`.

## Next

- Stage 5 picks up the next item on the SCOPE Phase 1 path (likely store / PrefsStore trait + sqlite impl per the stage-3 scaffold). A fresh session will start it.

## What you need to know

- The ICU-driven locale-default branch for display fields (DateFormat/TimeFormat/WeekStart/NumberFormat) and timezone is stubbed — `Auto` collapses straight to the hardcoded system default. Wiring lands when the i18n crate's ICU integration arrives.
- `SystemDefaults::starter()` matches SCOPE R3's hardcoded defaults: en-US, UTC, en, metric, ISO date, 24h, Monday, 1,234.56, USD, system theme.
- `StringPref::parse("auto") → Auto` is the helper the store layer (stage 6) should call when reading the free-string `timezone` / `currency` columns.
- The resolver's `Auto` semantics: an explicit `Auto` at any layer is *not* treated as a value — it falls through to derivation, matching R3's "explicit value at any layer" wording.

## Open questions

- (none)
