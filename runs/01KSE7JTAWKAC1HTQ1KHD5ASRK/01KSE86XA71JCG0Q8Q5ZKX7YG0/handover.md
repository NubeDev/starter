## Done

- Stage A.2 complete: 7 sequential commits (b7edc84 → 77bd5ca) landing G1–G7 binding substrate fixes in starter-ui-ir + starter-ui-bindings.
- C1: `visit_bindings` added to `Bindable` trait; per-variant dispatch on `Component`.
- C2: `substitute_tree` walks via `visit_bindings` (error stashed in local Option).
- C3: `Qualifier::{Default,Optional,Required}` on `Binding`, parsed from trailing `?` / `!`; optional swallows lookup errors.
- C4: `Source::Item`/`Source::Index`, `item`/`index` on `EvalContext`, new `expand.rs::expand_repeats`.
- C5: `Component::synthetic_id(parent_id, index)` + `assign_synthetic_id`, wired into expander.
- C6: `Component::is_portable()` const fn + `IR_PORTABLE_VARIANTS` exported.
- C7: `Source::Msg`, new `catalogue.rs` with `MessageBag`/`NullBag`, `catalogue`+`locale` on `EvalContext`, `BindingError::UnknownMessage`.
- `cargo test -p starter-ui-bindings -p starter-ui-ir` green after every commit.

## Next

- Stage 3 picks up the next phase per `rubix/docs/scope/dashboards/README.md` (host glue — 03-host-glue.md: 4 trait impls). Resolver still needs to thread request locale + MessageBag into `EvalContext`.

## What you need to know

- `crates/starter-ui-ir/src/component.rs` is now ~3200 lines (hand-written Bindable dispatch). Stage prompt explicitly permitted this; the doc plan to split into `bindable/<variant>.rs` is deferred to v2.
- `EvalContext` literal-construction sites in `starter-sdui-routes` and `starter-ui-builder` were updated for the new fields (`item`, `index`, `catalogue`, `locale`) with no behaviour change — production code still uses `&NullBag` and `"en"`.
- Per-stage spec deliberately diverges from `02-bindings-gaps.md`: G2 here is `?`/`!` qualifier (not prefix/suffix/default); G4 here is synthetic_id helper (not layout-override on ResolveRequest); G5 here is per-variant const fn (in addition to the const list).

## Open questions

- (none)
