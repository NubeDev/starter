## Done

- Added `starter-spi::units` module: closed `Quantity` enum (Temperature, Pressure, Speed, Length, Mass), closed `Unit` enum with the 13 lowercase wire codes locked in stage 1, `QuantityDef`, `UnitRegistry` trait, `StaticRegistry` impl, and `normalize_for_storage()` delegating all conversions to `uom`.
- Added `uom = "0.36"` to workspace `[workspace.dependencies]` and to `starter-spi`'s `[dependencies]` (only crate that names `uom` per R4).
- `cargo test -p starter-spi --lib` passes (13/13); `cargo clippy -p starter-spi --all-targets -- -D warnings` clean.
- Committed as `cf0de26` on branch `codeless/starter-prefs-spi`.

## Next

- Stage 4 of 7 — starter-spi::i18n module (LanguageTag, MessageKey, Diagnostic, DiagnosticParam). Picked up in a fresh session.

## What you need to know

- `uom 0.36` (not 0.38) is what cargo's MSRV/resolver picked; left it as `version = "0.36"` in workspace deps. `uom`'s pound is 0.4535924 kg (7 sig figs) — the pound→kg test relaxes to `1e-4` and a code comment explains why; bumping `uom` later may tighten this.
- `Quantity` and `Unit` are intentionally NOT `#[non_exhaustive]` — directive explicitly calls out that `#[non_exhaustive]` defeats the R4 closed-enum guarantee. Module docs quote R4 verbatim.
- `QuantityDef::allowed_units` is `&'static [Unit]` (compile-time registry) rather than `Vec<Unit>`; directive permitted either, this keeps `StaticRegistry` zero-alloc.
- Errors live in a local `UnitError` (thiserror) inside the units module rather than `crate::error::Error`, to keep the new module self-contained — re-route into the crate `Error` if Phase 1 needs it.

## Open questions

- (none)
