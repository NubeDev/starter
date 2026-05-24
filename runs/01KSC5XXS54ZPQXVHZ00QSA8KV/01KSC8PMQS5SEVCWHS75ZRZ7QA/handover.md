## Done

- Added `rubix/crates/rubix-agent/tests/goal_2_user_admin_test.rs` with two scenarios driving the user-admin verbs through `UndoDispatcher` + `UndoLastTool` end-to-end against an ephemeral SQLite changelog: (1) `create_via_dispatcher_persists_row_and_emits_diagnostic` asserts the row in the backing store + `rubix.user.created` Diagnostic + one Op::Create change recorded for the actor; (2) `undo_last_reverses_user_disable_back_to_enabled` creates+disables, asserts `disabled_at_ms` flips, fires `rubix.undo.last`, asserts the user is enabled again via `UserReversible::apply_inverse`. Both green.
- Extended `rubix/docs/design/user-admin/README.md` to cover the six bound verbs (added `rubix.user.list` and `rubix.tenant.list` rows + the two read MessageKeys) and pointed at the new integration test.
- Stripped the `rubix/SCOPE.md` reference from `rubix-store-postgres/src/lib.rs` §"Scope" so `rubix/scripts/lint-doc-refs.sh` is clean (pre-existing failure noted by stage 6 handover).
- `cargo test -p rubix-agent` → all green (mcp_stdio 3 ok / 1 ignored, rest_disk 4 ok, undo_dispatch 1 ok, undo_sweep 1 ignored (Docker), goal_2_user_admin 2 ok, plus the rest of the suite). `lint-doc-refs` clean.
- Two commits: `e50690a` test(rubix-agent), `c07f19c` docs(design) (folds the lib.rs lint fix).

## Next

- Stage 8 / Phase C.1 — Goal 4 clickhouse-ruler verbs (`clickhouse/rule_write.rs`, `clickhouse/mart_create.rs`, `clickhouse/retention_set.rs`) per SCOPE Phase C; each writes via `ChClient`, implements `Reversible` snapshotting prior state into the `undo_snapshots` table.

## What you need to know

- The integration test does NOT drive through the `rubix-admin mcp` stdio transport — the user verbs aren't yet wired into `boot::mcp::register::build_flow_registry`/`registry::build_tool_registry`. The test header documents this and the choice to exercise the `UndoDispatcher` seam (the equivalent layer the agent loop reaches once verbs are registered). When boot wiring lands in a later stage, layering an MCP-stdio assertion on top is straightforward.
- "PG row" assertion uses `InMemoryUserStore` (the only `UserAdminStore` impl today). The trait is the contract — production swap is one line in the boot wiring.
- `ChangeFilter` has no `actor` field; use `starter_changelog::filter_for_actor(&Actor)`.
- Scenario 2 reads the stage prompt's "undo + assert disabled" as "exercise undo over a disable" — the verb walks the disable back, so the post-undo assertion is `disabled_at_ms.is_none()` (user is enabled). Test name + comments make this explicit.

## Open questions

- (none)
