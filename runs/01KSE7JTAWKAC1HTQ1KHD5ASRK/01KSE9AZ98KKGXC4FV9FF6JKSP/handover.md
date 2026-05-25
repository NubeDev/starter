## Done

- Reviewed Phase A diff (2fb4d58..HEAD): A.1 commit `2fb4d58` (dashboards_definitions + PgDashboardStore + authz, +1 test file 122 LOC) and seven A.2 commits — `b7edc84` G1 part 1 visit_bindings (visit_bindings.rs ~15 tests), `d560d68` G1 part 2 substitute dispatch (substitute_dispatch.rs ~5 tests), `10b8b83` G2 qualifier grammar (qualifier_grammar.rs ~5 tests), `f122f54` G3 Repeat `$item` (repeat_expand.rs ~5 tests + cross-file updates), `46b6fa2` G4 synthetic ids (synthetic_id.rs 3 tests), `817f456` G5 portable subset (portable_subset.rs 3 tests), `77bd5ca` G6 `$msg` source (msg_source.rs ~3 tests).
- Confirmed chart-template demo: `crates/starter-ui-bindings/tests/substitute_dispatch.rs::chart_source_node_id_substitutes_from_target` (line 84) constructs a Chart with `node_id = "{{$user.device}}"` and asserts the resolved value is `"dev-42"` — non-text widget resolves rather than rendering literally.
- `cargo test -p starter-ui-bindings -p starter-ui-ir` green; aggregate ~100+ assertions across the new test files.
- Verified Layer-1 invariants: R1 — `starter-ui-bindings`/`starter-ui-ir` Cargo.toml carry zero `rubix-*` deps (direction preserved, rubix-spi/rubix-store-postgres correctly sit above); R2 — substitute remains the sole evaluator in starter-ui-bindings, no second transport introduced; R4/R5 — bindings still evaluate server-side, no client-eval path added; wire formats — diff on `crates/starter-ui-ir/src/component.rs` is additive only (new `is_portable`, `synthetic_id`, `assign_synthetic_id`, `Bindable` impl), no variant fields renamed or removed.

## Next

- Phase B (next session): host-glue per `rubix/docs/scope/dashboards/03-host-glue.md` — implement `EntityGraph`/`PageProvider`/`QueryEngine`/`HandlerRegistry` in rubix-agent and wire `sdui_router`.

## What you need to know

- PASS: bindings substrate sound, R1/R2/R4/R5 intact, wire format additive-only, chart `node_id` template resolves end-to-end.
- No patches proposed at this gate per instructions.

## Open questions

- (none)
