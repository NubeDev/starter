## Done

- Added `packages/starter-sdui-react` as a workspace package depending on react, @tanstack/react-query, zustand, @nube/starter-ui-kit, @nube/starter-ui-core.
- Ported the rubix-ui-core/src/sdui/ surface verbatim in shape: SduiProvider/context, Renderer.tsx (68 lines, well inside the 800-line CI budget), SduiPage, SduiRenderPage, SduiDialogHost, types.ts, registry/{types,index}.ts, applyPatch, useActionResponse, useSubscriptions, useBoundWrite, row-bind, dialog-bus, capability, show-when.
- Implemented 19 component specs (page, row, col, grid, stack, tabs, card, text, heading, badge, kpi, kpi_grid, button, link, table, form, field, select, toggle, plus the `custom` escape hatch) against @nube/starter-ui-kit shadcn primitives — ~960 lines total, comfortably inside the 3000/4000 budget.
- Wired `registerCustomRenderer(kind, component)` to the module-level `globalCustomRegistry`; the `custom` spec dispatches against it.
- Capability handshake: `SUPPORTED_IR_VERSION = 5` + `checkIrVersion(tree)`. Both SduiPage and SduiRenderPage refuse to project when `ir_version > SUPPORTED_IR_VERSION` and render a mismatch banner (R2).
- Diagnostics flow is the wider `{ severity, code, message, field? }` shape (D1 already lived in the IR); forms intercept the `diagnostics` action variant and render inline + banner.
- `pnpm --filter @nube/starter-sdui-react run typecheck` is green.
- Committed: `Phase 4 -- @nube/starter-sdui-react port. …`

## Next

- Stage 7 (Phase 5 -- `starter-sdui-routes` reference HTTP routes + the structural domain-leak allowlist enforcing R3, server-side DoS limits R8, RSQL table queries R6, single /ui/action endpoint R5).

## What you need to know

- D2 (render against starter-ui-kit shadcn, not @rubix/ui-core) was already recorded in DOCS/frontend/sdui/DIVERGENCE.md from stage 2 (Phase 1) and matches what this stage required — no further drift entry was needed. No new divergences introduced.
- The package consumes `@nube/starter-ui-kit` source files; ui-kit's own files use `@/lib/utils` etc. The new package's `tsconfig.json` adds a second `paths` entry (`../starter-ui-kit/src/*`) so tsc can resolve those imports when typechecking the consumer; this mirrors the pattern other sibling packages would need if/when they import shadcn primitives from source.
- The renderer is host-transport-agnostic: SduiPage takes `resolve` and `dispatchAction` props (HTTP plumbing is the host's job), and useSubscriptions takes an optional `SubscriptionTransport` (SSE/WS/polling is also the host's). This keeps the package zero-I/O per the package description.
- A pre-existing typecheck failure in `starter-extensions/examples/notes` (StarterClient.get/post) is unrelated to this stage and was present before.
- The `Renderer` deliberately does NOT carry visual-builder drop-zone logic from Rubix — that lives in a (future) builder pane outside this package; `RendererList` accepts `parentId`/`parentType` but treats them as inert, leaving room for the pane to opt in later without changing the spec API.
- The IR-version constant is `5` (inherited from Rubix at port time, per the DIVERGENCE.md "Reserved" note). Bumping when starter ships a variant Rubix doesn't have requires a new D-entry per the divergence doc's guidance.

## Open questions

- (none)
