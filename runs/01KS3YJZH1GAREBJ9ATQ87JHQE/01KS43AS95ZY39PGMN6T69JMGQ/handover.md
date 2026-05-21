## Done

- New `packages/starter-sdui-react/src/useOptimisticAction.ts` with a pure `runOptimisticDispatch(qc, treeQueryKey, dispatchAction, interpret, handler, args, optimistic)` helper under a thin React hook. Apply pre-dispatch via `mergeAt`; on round-trip throw → restore snapshot + re-throw; on `diagnostics` reply → restore snapshot then `interpret`; on `patch` / `full_render` → `interpret` (which replaces via the same `applyPatch` helpers).
- `Button` (Interactive.tsx) now reads `node.optimistic` and dispatches through `useOptimisticAction`; the legacy `useActionResponse` path stays for non-action surfaces (links, dialog interpreter).
- `types.ts` adds `OptimisticHint { target_component_id, fields }` mirroring `starter_ui_ir::OptimisticHint`; both re-exported from `index.ts`.
- `crates/starter-ui-ir/src/lib.rs` now re-exports `OptimisticHint` at crate root so authoring code can `use starter_ui_ir::OptimisticHint`.
- `crates/starter-ui-builder/tests/falsification_three_pages.rs` builds three fixtures (CRUD device list, PR review card with diff + optimistic approve/request_changes buttons, scope board with KPI tiles + status badges + live table). Each asserts: serde round-trip, every emitted `type` lives in the renderer's built-in dispatch table, and (review card) the `Action.optimistic.target_component_id` + `fields.disabled` are present in the wire JSON. A fourth test unions the three fixtures and pins that they all share one dispatch path.
- `packages/starter-sdui-react/src/r3-no-domain-leak.test.ts` scans every `.ts` / `.tsx` under `src/` (excluding test files + itself) for a denylist of fixture-domain terms — defence-in-depth tripwire on top of the SCOPE-mandated `words.txt` allowlist.
- `packages/starter-sdui-react/src/useOptimisticAction.test.ts` pins: optimistic applied pre-resolve, authoritative patch replaces, diagnostics rolls back, dispatch throw rolls back + re-throws.

## Next

- Stage 12 (final stage) — likely the scrub plan trigger check / Phase-9-style closeout. A fresh session will pick it up per the job model.

## What you need to know

- The seven R8 DoS limits already had stable `what:` tag tests in `crates/starter-sdui-routes/tests/limits_413.rs` from stage 8 (page_state_bytes / render_tree_bytes / tree_nodes / tree_depth / component_types / handler_timeout / table_rows_per_page). This stage did NOT add a new smoke suite for them — confirmed via the existing test file. The SCOPE R8 table already carries the evidence column (Inherited / unmeasured / Reused) for every row per the M3 fix; no edit to SCOPE.md was needed.
- `form_errors` rejection at the wire was already enforced in `crates/starter-ui-ir/src/action.rs::form_errors_tag_is_rejected_at_the_wire` from stage 2 (D1). The Phase 8 contract adds the rollback-on-diagnostics behaviour, which is new.
- The `OptimisticHint` field-name choice matches Rubix: `target_component_id` + `fields` (NOT `id` + `patch`) — same wire on both sides of the Rust↔TS boundary.
- Vitest runs in `environment: "node"` (no jsdom) so the hook test exercises the extracted pure helper via a `QueryClient` instance + fake `dispatchAction` + lightweight interpret stand-in. The real `useActionResponse` is re-exported and smoke-checked for presence.
- `DiffAnnotation` only has `{ line, text, author, created_at }` — no `side`/`severity`/`message` fields. The fixture uses the actual struct shape.

## Open questions

- (none)
