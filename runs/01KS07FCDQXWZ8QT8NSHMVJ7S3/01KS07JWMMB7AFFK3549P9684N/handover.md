## Done

- Added a "Phase 0 wire boundary (locked)" subsection under "Decisions made" in DOCS/user/scope/SCOPE.md capturing items (a)–(k) from stage 1 with revisit triggers for each rule.
- Confirmed Quantity v1 = {Temperature, Pressure, Speed, Length, Mass}; Ratio explicitly deferred (no Preferences column needs it; Percent stays display-only; {Ratio,Percent} rejection lands in Phase 1 resolver once Ratio is introduced).
- Confirmed Unit v1 = {C, F, Kpa, Psi, Bar, MPerS, KmPerH, Mph, Knot, M, Ft, Kg, Lb}; canonical SI per R1 = C / Kpa / MPerS / M / Kg.
- Money kept out of Unit (i64 minor-units + ISO 4217). Theme={Light,Dark,System} user-only. DateFormat={Auto,IsoYMD,DmySlash,MdySlash}. TimeFormat={Auto,H24,H12}. WeekStart={Auto,Monday,Sunday}. NumberFormat={Auto,CommaDot,DotComma,SpaceComma}. UnitSystem={Metric,Imperial}.
- Committed as 984a262 on branch codeless/starter-prefs-spi.

## Next

- Stage 2 (next session) begins code: create starter-spi modules `preferences`, `units`, `i18n` with the enums and DTOs locked here. `normalize_for_storage(Quantity, Unit, f64) -> f64` lives in `starter-spi::units`.

## What you need to know

- No code was added in this stage by design — stage 1 is decision-only.
- Variant naming chosen for Rust: MPerS, KmPerH (avoid leading digits / slashes). Wire format string spelling (e.g. "m/s") is a separate Serde decision for stage 2.
- The {Ratio, Percent} registry-rejection rule is documented but enforcement is intentionally deferred to Phase 1 because Ratio is not in v1 Quantity.
- Workspace ceilings (R1–R8, ≤400 lines/file, ≤~10 public items/module, no utils/helpers/common) apply to upcoming code.

## Open questions

- (none)
