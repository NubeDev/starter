## Done

- Added `starter_spi::units::UnitMetadata` ({quantity, unit}) with Serialize/Deserialize/ToSchema in `crates/starter-spi/src/units/metadata.rs`, re-exported from `units` module.
- Added `starter_prefs::dto::series` with `SeriesEnvelope<T>` (slot, quantity, unit, points: Vec<(i64,T)>), `SeriesPoint<T>` alias, and `ToCanonicalSeries` / `FromCanonicalSeries` adapter traits. Derives Serialize+Deserialize+ToSchema; openapi shape for `points` overridden to `Vec<Vec<Value>>` so the JSON array tuple wire form is reflected.
- Tests cover R8 exact wire shape match, serde_json round-trip, metadata-hoisting (no per-point quantity/unit), ToSchema openapi field/required presence, and adapter-trait round trip for a sample consumer type.
- `cargo test -p starter-spi` and `cargo test -p starter-prefs --features sqlite,routes` pass; `cargo build -p starter-server` clean.
- Committed as `stage 9 — Phase 2 per-series response shape per R8` (94baeaf).

## Next

- Stage 10 picks up the next item in WORKFLOW.md (likely starter-prefs canonical-only-logs audit / R7 CDN-cache docs, then on to Phase 3 starter-i18n crate).

## What you need to know

- utoipa 5 does not impl ToSchema for Rust tuples; I used `#[schema(value_type = Vec<Vec<serde_json::Value>>)]` on `SeriesEnvelope::points` to express the `[ts, value]` array shape in openapi.
- `#[aliases(...)]` from older utoipa is not in scope in this version — removed in favour of letting consumers name `SeriesEnvelope<f64>` directly.
- `UnitMetadata` lives in `starter-spi` (closed-enum scope owns the leaf pair); `SeriesEnvelope` lives in `starter-prefs::dto::series` per D-2.1 so openapi.json captures it from the routes crate that already contributes paths.
- Wire shape verified literally matches SCOPE §R8 JSON example.

## Open questions

- (none)
