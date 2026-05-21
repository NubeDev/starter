# `@nube/starter-sdui-react`

React renderer for starter Server-Driven UI trees.

Ports `rubix-ui-core/src/sdui/` verbatim in shape (`SduiProvider`,
`Renderer`, `SduiPage`, `SduiRenderPage`, `useActionResponse`,
`useSubscriptions`, `useBoundWrite`, `applyPatch`, `dialog-bus`,
`row-bind`, `capability`, `registry/`) and projects against
`@nube/starter-ui-kit` shadcn primitives — **not** `@rubix/ui-core`'s
primitives (divergence **D2**, see
[`DOCS/frontend/sdui/DIVERGENCE.md`](../../DOCS/frontend/sdui/DIVERGENCE.md)).

## What's here

- `SduiProvider` / `useSdui` — the renderer context carrying the
  action dispatcher, custom-renderer registry, page-state writer,
  React-Query tree key, and write plan.
- `Renderer` — single-file `node.type` → `ComponentSpec` dispatcher.
  Size-budgeted to ≤ 800 lines TSX (CI gate).
- `SduiPage` / `SduiRenderPage` — page-level wrappers that resolve
  via `POST /api/v1/ui/resolve` (or accept a pre-resolved tree),
  check the IR-version capability handshake, and mount the renderer.
- `capability.ts` — `SUPPORTED_IR_VERSION` constant + `checkIrVersion`.
  The renderer refuses to project a tree whose `ir_version` exceeds
  it (R2).
- `applyPatch.ts` — `mergeAt` / `replaceAt` for optimistic action
  hints and authoritative Patch / FullRender responses.
- `useActionResponse.ts` — discriminator for `UiActionResponse`
  (`toast`, `redirect`, `patch`, `full_render`, `dialog`,
  `dismiss_dialog`, `diagnostics`, …). Diagnostics is the wider
  `{ severity, code, message, field? }` shape per divergence **D1**.
- `useSubscriptions.ts` — subscription-plan executor; rebinds slot
  writes back into the cached tree under `treeQueryKey`.
- `useBoundWrite.ts` — two-way binding hook (controls look up the
  write plan, dispatch action, write optimistically).
- `row-bind.ts` — `{{$row.*}}` template substitution for table-row
  child trees.
- `dialog-bus.ts` — module-level LIFO stack for dynamic dialogs.
- `registry/` — `ComponentRegistry`, `ComponentSpec`, `Kind` union,
  `builtinComponentRegistry`, `registerCustomRenderer`.

## Component implementations

The Phase 4 batch ports the 19 initial component kinds against
shadcn primitives:

`page`, `row`, `col`, `grid`, `tabs`, `stack`, `card`, `text`,
`heading`, `badge`, `kpi`, `kpi_grid`, `button`, `link`, `table`,
`form`, `field`, `select`, `toggle`.

Total component implementation lines stay under a **3000 target /
4000 red line** budget enforced by CI.

## Registering custom renderers

Pre-render (e.g. plugin bootstrap):

```ts
import { registerCustomRenderer } from "@nube/starter-sdui-react";

registerCustomRenderer("com.acme.floorplan", FloorPlanComponent);
```

Custom renderers appear in trees as
`{ "type": "custom", "renderer_id": "com.acme.floorplan", "props": { ... },
"subscribe": [ ... ] }`. The `renderer_id` is the lookup key in the
registry — match it exactly to the string passed to
`registerCustomRenderer`.

## Custom is a reference, not a node

`type: "custom"` is the **escape hatch**, not a new IR variant. The
distinction matters:

- An **IR component** (`page`, `card`, `chart`, `form`, …) is part
  of the wire vocabulary. It has a schema in `starter-ui-ir`; the
  binding engine walks its fields; the capability filter checks its
  shape; the builder DSL has a typed constructor for it.
- A **`custom` node** carries a `renderer_id` plus opaque `props`. The
  IR does not type-check `props`; the binding engine does not walk
  them; the capability filter checks only the `renderer_id` (R7). The
  renderer **dereferences** `renderer_id` against the consumer-owned
  custom registry — the actual rendering is whatever React component
  the consumer registered. The IR did not grow a new variant; the
  client just dispatched to user code.

Practical consequences:

- Authoring a new IR component means adding a `Component` enum
  variant in `starter-ui-ir`, a `ComponentSpec` here, and (usually) a
  builder constructor. It changes the IR's vocabulary, which is a
  schema-versioned change (R2).
- Adding a `custom` renderer is **local** — one
  `registerCustomRenderer` call. No IR change, no version bump.
- A `custom` node whose `renderer_id` the client does not know
  renders the **fallback stub** (a neutral placeholder div) and logs
  a structured `sdui.custom.unknown_renderer` warning. The rest of
  the tree renders normally; one unknown id never takes down a page.
  The server-side capability filter is supposed to rewrite unknown
  ids to a `dangling` stub before emission (see
  `starter-sdui-routes::capability`); the client-side fallback is the
  belt-and-braces second line of defence.

### Authorisation boundary

`custom.props` are scoped to the **renderer's contract**, not to the
user's permissions. The capability filter is a *vocabulary* check
("does this client know how to render this id"), never an
*authorisation* check. Per-principal authorisation runs at the
handler boundary (R5) and at the `/resolve` boundary, both
**before** any `custom` node is constructed — see the threat-model
section in the `starter-sdui-routes` crate-level docs.

## Capability handshake (R2)

`SUPPORTED_IR_VERSION` is the highest `ir_version` this renderer
projects. On every page resolution, `checkIrVersion(tree)` runs
before mount; a tree with a higher `ir_version` renders a
mismatch banner and refuses to project — bumping the renderer's
support window is the only fix. Lower versions are accepted
(the server clamped emission).
