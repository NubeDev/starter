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

registerCustomRenderer("my_block", MyBlockComponent);
```

Custom renderers appear in trees as
`{ "type": "custom", "kind": "my_block", "props": { ... }, "subscribe": [ ... ] }`.

## Capability handshake (R2)

`SUPPORTED_IR_VERSION` is the highest `ir_version` this renderer
projects. On every page resolution, `checkIrVersion(tree)` runs
before mount; a tree with a higher `ir_version` renders a
mismatch banner and refuses to project — bumping the renderer's
support window is the only fix. Lower versions are accepted
(the server clamped emission).
