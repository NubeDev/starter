# 2026-05-25 — Flow editor node position persistence landed

Closing note for the third pass over the rubix flow editor
round-trip. The first pass made nodes render and delete
in place; the second pass made handle-to-handle edge
connections survive a hard reload; this pass makes
**node spatial positions** survive a hard reload. The
operator can now drag the `count` node, refresh the page,
and find it exactly where they left it — because the
canvas position is the source of truth for itself and is
written back into the flow YAML on drag-end.

## Why positions belong in YAML

Up to this branch `yamlToGraph` auto-laid out every node in
a single column (`{ x: 80 + i * 280, y: 160 }`) every time
the route mounted. Dragging worked locally but the next
reload reset the canvas. That is fine for a request-shaped
demo flow and wrong for everything else: as soon as an
operator has more than three nodes the layout is *their*
layout, not the engine's. The position belongs to the flow
the same way `kind`, `slots`, and `links` do — it is
operator intent that the engine should preserve across
process restarts, hot-edits, and `flow_ops.deploy` calls.

The shape is the minimum that survives serde round-trips
and the hot-reload classifier:

```rust
// crates/starter-flow/src/definition/body.rs
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NodePosition {
    /// X coordinate in canvas-space pixels.
    pub x: f64,
    /// Y coordinate in canvas-space pixels.
    pub y: f64,
}
```

`NodeDecl.position` is `Option<NodePosition>` with
`#[serde(default, skip_serializing_if = "Option::is_none")]`
so flows authored before this branch keep parsing and keep
serialising without spurious empty `position: null` lines.
The structural-delta classifier is untouched — position is
not part of the `(id, kind, trigger-set)` topology key, so a
drag triggers a Settings hot-reload, not a topology swap.

## What landed where

### Backend — typed projection

- [crates/starter-flow/src/definition/body.rs](../../../crates/starter-flow/src/definition/body.rs)
  adds `NodePosition` and the optional field on `NodeDecl`.
  `NodeDecl::new` defaults to `None` so every existing
  call site compiles unchanged.
- [crates/starter-flow/src/definition/resolver.rs](../../../crates/starter-flow/src/definition/resolver.rs)
  destructure updated to ignore the new field — it does
  not affect resolution because positions are presentation,
  not topology.

### Frontend — canvas ⇄ YAML round-trip

- [rubix/frontend/src/routes/flows/$flowId.tsx](../../frontend/src/routes/flows/$flowId.tsx)
  extends `ParsedFlow.nodes` with optional position;
  `yamlToGraph` honours a persisted `position` when the
  YAML has it and falls back to the auto-layout column
  only for fresh nodes that have never been dragged.
- [rubix/frontend/src/lib/sync-flow-graph.ts](../../frontend/src/lib/sync-flow-graph.ts)
  in Section 1 now (a) prunes removed nodes and (b)
  writes back rounded `position: { x, y }` for surviving
  nodes — inline-flow style, only when the value
  actually changed, so the `yaml` library's preserved
  comments and field ordering are not disturbed.
- [packages/starter-ui-flow/src/hooks/useFlowGraph.ts](../../../packages/starter-ui-flow/src/hooks/useFlowGraph.ts)
  promotes a position change to a persistent change *only*
  on drag-end (`c.type === "position" && c.dragging === false`).
  Intermediate drag frames stay local so we do not
  deploy 60 YAML mutations a second across a single drag
  gesture.

## E2E proof — the assertion that matters

The Playwright spec
[rubix/frontend/e2e/flow-editor-roundtrip.spec.ts](../../frontend/e2e/flow-editor-roundtrip.spec.ts)
gains a third test, "persists node positions across
reloads". Two earlier attempts asserted on screen-space
bounding boxes and both failed for the same reason:
xyflow's `fitView` re-runs on mount with whatever viewport
the browser is sized to, so the pixel coordinates of a
node drift ~48px between mounts even when the underlying
flow data is identical.

The fix is to assert on the source of truth instead. The
test reads the flow body straight out of
`rubix.flow_ops.list`, regex-extracts the persisted
position of the `count` node, reloads the page, reads the
position again, and asserts the two are equal:

```ts
const persisted = (await readCountPos())!
await page.reload()
// …re-wait for canvas…
const afterReload = (await readCountPos())!
expect(afterReload).toEqual(persisted)
```

All three round-trip tests pass on chromium:

```
✓ renders handles for all built-in kinds and deletes in place (7.7s)
✓ connects two handles by drag and persists the new edge (7.3s)
✓ persists node positions across reloads (8.3s)
3 passed (24.9s)
```

## Operator-visible behaviour now

1. Open `/flows/<id>`, drag the `count` node 200px right.
2. The canvas commits the move on mouse-up; one
   `flow_ops.deploy` fires (debounced via
   `pendingChangeRef`), not one per frame.
3. The YAML in postgres now carries
   `position: { x: 280, y: 160 }` on the `count` node.
4. Hard-reload the page. The node renders at
   `x: 280, y: 160`, not back at the auto-layout column.
5. Restart `rubix-agent`. Same.

## What is still open from the Phase 3 list

- **Surface node slot values on the canvas.**
  `BaseNode.SlotRow` already reads from
  `RunOverlay.slotValues`; needs an e2e that proves the
  badges fill in during a live tick. Investigation should
  start from the `examples/admin/src` reference the
  operator cited.
- **Edge deletion.** xyflow emits an `EdgeChange` of
  type `remove` on Backspace when an edge is selected and
  the handler is already wired through
  `lastGraphRef.current`. Needs an e2e: select an edge,
  press Backspace, assert the YAML `links:` count
  decreased and the link is not in `links:` after a
  reload.

Both are framed and ready to pick up; this note closes
the position pass cleanly so the next pass starts from a
known-green tree.
