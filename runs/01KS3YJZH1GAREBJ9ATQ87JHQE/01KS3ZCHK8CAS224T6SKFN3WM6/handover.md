## Done

- Created `crates/starter-ui-bindings` (deps: `starter-ui-ir` + serde/serde_json/thiserror/tracing only — inherits R1's no-I/O posture).
- Ported the binding grammar verbatim (semantics) from Rubix dashboard-runtime: `$target` / `$stack.alias` / `$self` / `$user` / `$page` sources, `.` slot-read, `/` child-walk, with length-prefixed step-list evaluation.
- Defined the `EntityGraph` trait (S-D1: stays in `starter-ui-bindings` until a second consumer) with `read_slot`, `read_children`, and optional `entity_id_regex` (returns `None` by default — consumed by ai-builder R7). Ships a `NullGraph` impl for `$user`-only / placeholder use.
- Implemented `EvalContext` with optional `access_log: &RefCell<Vec<SlotAccess>>`, and `SubscriptionPlan::from_log` that dedupes + sorts into NATS-ish `entity/slot` subjects.
- Added `substitute_text` / `substitute_tree` helpers so a `ComponentTree` with `{{...}}` tags can be substituted in place over `Component::Text` / `Heading` content (full per-variant Bindable dispatch is Phase 3+ work).
- `tests/one_page_n_targets.rs` — the Phase 2 acceptance test passes: one page, three targets, three distinct literals, three disjoint subscription plans.
- 15 inline unit tests + 2 integration tests all green (`cargo test -p starter-ui-bindings`).
- Workspace `Cargo.toml` updated (members + workspace dep).
- `DOCS/frontend/sdui/DIVERGENCE.md` D5 augmented with a Phase 2 landing note (trait methods, `NullGraph`, S-D1 revisit trigger). The D5 row itself was pre-seeded by an earlier stage; this stage adds the concrete shape.
- Committed as `Phase 2 -- starter-ui-bindings port` on branch `codeless/starter-sdui`.

## Next

- (none) — Stage 5 (the next session) picks up `starter-ui-builder` per SCOPE.md Phase 3.

## What you need to know

- The grammar in SCOPE.md is the simpler `$target/child.slot` form, not Rubix's nodeRef ref-walk shape. Where the two disagreed I followed SCOPE.md verbatim (with `EntityGraph` as the host-supplied seam — that *is* D5). Rubix's `dashboard-runtime` binding.rs has additional sources (`$vars`, `$item`, `$index`, `$node:<uuid>`, `$<alias>`) which were **not** ported in this phase — they are not in the SCOPE.md grammar and no stage instruction asked for them. If a later stage needs them, extend `Source` / `parse` / `seed`.
- "Length-prefixed evaluation" was interpreted as: each step in the parsed `Binding` carries its operator prefix (`.` or `/`) and the evaluator walks left-to-right, so step *N*'s cursor is fully determined by steps `0..N`. This is what makes the "one page, N targets" property hold — the parsed expression has no per-target state.
- The substitute helper only walks `Page`/`Row`/`Col` containers and `Text`/`Heading` content fields. Other variants pass through unchanged in Phase 2; the renderer (Phase 4) and builder (Phase 3) will lean on the IR's existing `Bindable` trait for full per-variant coverage.
- `Subject::wire()` formats as `"<entity_id>/<slot>"` — a host with a different subject grammar (dot-separated, hierarchical) is free to format its own.
- `EvalContext::new(graph)` is a convenience that leaks two `OnceLock`-backed empty maps for the `stack`/`user`/`page` borrows. Fine for tests; production resolvers will pass populated maps.

## Open questions

- (none)
