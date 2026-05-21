## Done

- Added `crates/starter-ui-builder` as a workspace member depending only on `starter-ui-ir` (plus serde/serde_json/thiserror).
- Ported the typed Rust constructors verbatim: `page`, `row`, `col`, `grid`, `tabs`, `card`, `heading`, `text`, `badge`, `kpi`, `kpi_grid`, `table`, `line_chart`, `bar_chart`, `gauge`, `sparkline`, `form`, `select`, `toggle`, `slider`, `date_range`, `ref_picker`, `action_form`, `dashboard`. `TimeSeriesSource` / `RowsSource` newtypes give compile-time chart/source pairing. `rsql()` builder ported. `bindings::{target, stack, user, self_, page_state, vars}` helpers ported.
- `seed_page()` ported with an idempotent upsert contract abstracted behind a new `PageStore` trait (synchronous, two methods). Logged as divergence D7 in `DOCS/frontend/sdui/DIVERGENCE.md`.
- Crate-level docs (`src/lib.rs`) carry the compile-time-vs-resolve-time contract verbatim from `SCOPE.md`.
- `tests/worked_example.rs` — a `dashboard()` + `kpi_grid()` + `table()` page authored from `main.rs` resolves end-to-end against the Phase 2 fixture entity graph; per-target subjects scope subscriptions across three resolves.
- `tests/builder_smoke.rs` — every public builder function round-trips through serde and, for variants whose schemars output matches the wire shape, the IR JSON Schema artifact.
- `cargo test -p starter-ui-ir -p starter-ui-bindings -p starter-ui-builder` all green; `cargo tree -p starter-ui-builder --edges normal` confirms no axum/reqwest/tokio/etc.
- Commit `673ba7f` on `codeless/starter-sdui` titled `Phase 3 -- starter-ui-builder port`.

## Next

- (none — fresh session picks up Stage 6.)

## What you need to know

- Two `Validate` modes in `builder_smoke.rs`: `SchemaAndSerde` runs the JSON Schema validator against `crates/starter-ui-ir/schema/starter-ui-ir.schema.json`; `SerdeOnly` is used for variants touching `Bindings` (custom `Serialize` unwraps single-spec to a bare string/object) or `ChartKind` (custom `Serialize` emits snake_case while schemars emits PascalCase variant names). Both are Phase 1 schemars-derivation gaps — when the schema artifact is reconciled with the runtime wire, the allowlist collapses to one path.
- Phase 2's `substitute_tree` only walks `Text.content` / `Heading.content`. The worked-example test pins the page-title binding as untouched on the wire (asserting `"{{$target.name}} Overview"`) so a future widening of substitute coverage flips the assertion intentionally.
- `seed_page` is synchronous; callers wrap an async store at the call site. The idempotency contract is on the `PageStore::find_or_create_node` impl, not on the function — implementations must reuse existing nodes atomically (Rubix's `SEED-RECONCILE.md` bug).
- The builder explicitly does NOT depend on `starter-ui-bindings`; the smoke + worked-example test pull it as a dev-dep only, to exercise the resolve-time path against the Phase 2 fixture.
- `bindings.rs` ports only the simple slot-read helpers (`target("name")` → `{{$target.name}}`); the child-walk `/` grammar is expressed by appending to the returned string (e.g. `"Temp: {{$target/temp.value}}"`).

## Open questions

- The IR JSON Schema artifact and the runtime `Serialize` impls diverge for `Bindings` and `ChartKind`. The smoke test routes around it today; a follow-up should add schemars overrides (or hand-written `JsonSchema` impls) in `starter-ui-ir` so every variant validates under one path.
