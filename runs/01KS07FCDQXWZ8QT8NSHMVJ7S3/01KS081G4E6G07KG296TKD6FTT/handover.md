## Done

- Added `crates/starter-spi/src/preferences/` directory module with one-file-per-type layout: `unit_system.rs` (Metric/Imperial), `theme.rs` (Light/Dark/System), `date_format.rs` (Auto/IsoYMD/DmySlash/MdySlash), `time_format.rs` (Auto/H24/H12), `week_start.rs` (Auto/Monday/Sunday), `number_format.rs` (Auto/CommaDot/DotComma/SpaceComma), `resolved.rs` (`ResolvedPreferences`, no Option / no "auto"), `patch.rs` (`PreferencesPatch`, all Option<T> with `skip_serializing_if`), plus `tests.rs`.
- Wire spellings pinned to SCOPE column-comment strings byte-for-byte; snake_case where it matches, explicit `#[serde(rename = "...")]` for `YYYY-MM-DD`, `DD/MM/YYYY`, `MM/DD/YYYY`, `24h`, `12h`, `1,234.56`, `1.234,56`, `1 234,56`.
- All types derive `serde` + `utoipa::ToSchema` per R7. Added `ToSchema` to `units::Unit` and `units::Quantity` so the recursive schema lands.
- Registered `pub mod preferences;` in `crates/starter-spi/src/lib.rs`.
- `cargo test -p starter-spi --lib` green (27/27, 14 new); `cargo clippy -p starter-spi --all-targets -- -D warnings` green.
- Committed as `281843d` with a message starting `stage 4 — starter-spi::preferences module.`

## Next

- Stage 5: `starter-spi::i18n` module (LanguageTag, MessageKey, Diagnostic, DiagnosticParam).

## What you need to know

- Module layout follows `units/` (file-per-type + `mod.rs` barrel + `tests.rs`), not single-file.
- `PreferencesPatch` collapses missing-key and explicit-null to `None` here; the Phase 1 route layer is expected to disambiguate "leave alone" vs "revert to inherit" before reaching this DTO.
- `ResolvedPreferences` `Hash`/`Eq` derived — fine because every field type is `Hash + Eq` (no floats).
- `Unit` and `Quantity` now derive `ToSchema`; this is the only out-of-`preferences/` change in this stage and is mandatory for the containing struct schemas to compile.

## Open questions

- (none)
