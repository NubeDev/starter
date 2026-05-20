## Done

- Implemented `crates/starter-server/src/middleware/accept_units.rs` with `AcceptUnitsLayer` + `AcceptUnitsService<S>`, `accept_units_layer(registry, prefs)` constructor, `with_accept_units` router-wrapper helper, `UnitsCtx` (with `mode`/`prefs`/`user_unit`/`target_unit`/`convert`), `UnitsMode` (`Preferred` default, `Canonical`), and the `PrefsResolverFor` async trait.
- Header parsing: `Accept-Units: canonical` → suppress conversion (target = registry canonical); missing/`preferred`/unknown → user prefs. `Vary: Accept-Units` appended (idempotent, append-not-clobber).
- `UnitsCtx::convert(quantity, value, source_unit)`: source→canonical via `starter_spi::units::normalize_for_storage`; canonical→target via affine inverse sampled at 0.0/1.0 of `normalize_for_storage` (exact for every v1 conversion incl. °C/°F offset); identity short-circuit when target==canonical.
- Exposed in `middleware/mod.rs`. Added `http-body-util = "0.1"` to dev-deps.
- 6 unit/integration tests cover: default→preferred + Vary; canonical bypass; explicit preferred; UnitsCtx extension presence; round-trip math for Temperature/Length; Vary append alongside handler-set Vary. `cargo test -p starter-server` green.
- Committed as `stage 8 — Phase 2 Accept-Units tower middleware in starter-server` (c1bc53c).

## Next

- Stage 9 (per SCOPE): wire the resolved-prefs path end-to-end (`starter-prefs` adapter that implements `PrefsResolverFor` against `PrefsStore` + `SystemDefaults`, hooked into a starter-server builder option) and update typed serialisers / per-series wire shape per R8.

## What you need to know

- `PrefsResolverFor` lives in `starter-server::middleware::accept_units`, not `starter-prefs`, to avoid pulling axum into starter-prefs at this layer. The adapter that bridges to `starter-prefs::store::PrefsStore` + `resolver::resolve` is intentionally deferred to the next stage — this stage only ships the middleware seam and a `StubResolver` in tests.
- `UnitsCtx::target_unit` is the public seam typed serialisers should consult to decide what unit label to emit alongside a value; `convert` returns `(value, unit)` so both are in sync.
- The affine-inverse trick in `convert` assumes every conversion in `starter_spi::units::convert` is an affine map `y = a*x + b`. That holds for the entire v1 registry (uom-backed scales + a single zero-offset case for temperature). If a non-affine unit ever lands, the helper must be rewritten — flagged in the doc comment.
- A `from_fn`-based version would have been shorter but the concrete `Layer`/`Service` lets the type be named (`AcceptUnitsLayer`) and re-exported, matching what the SCOPE Phase 2 spec calls for.

## Open questions

- Whether `PrefsResolverFor` should ultimately move to `starter-spi` (to let other middlewares — i18n, theme — share the "resolve prefs once per request" pattern) or stay in `starter-server`. Punted; revisit when Phase 3 wires Accept-Language.
- `UnitsMode::parse` silently falls back to `Preferred` on unknown values (defensive). The SCOPE doesn't pin reject-vs-fallback behavior — flagged here in case Phase 2 review wants 415 instead.
