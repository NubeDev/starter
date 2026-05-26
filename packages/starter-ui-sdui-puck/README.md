# @nube/starter-ui-sdui-puck

Visual SDUI editor powered by [Puck](https://github.com/puckeditor/puck).
Generates a Puck `Config` from the committed
[`starter-ui-ir` JSON Schema](../../crates/starter-ui-ir/schema/starter-ui-ir.schema.json)
and renders a canvas + palette over the same widget vocabulary the
AI builder emits and `@nube/starter-ui-sdui-react` renders.

This package implements PR1 of the Puck builder per
[`rubix/docs/scope/dashboards/10-puck-builder.md`](../../rubix/docs/scope/dashboards/10-puck-builder.md).
Scope §B1 / §B2 / §B6 (CI portion only) land here; the save path
(§B4), data-source selectors (§B3), the dashboard route (§B5), and
the live-canvas banner (scope 11) ship in subsequent PRs.

## Public exports

```ts
import {
  buildPuckConfig, // pure (schema, slots, overrides, bindable, taxonomy) → Config
  PuckBuilder,     // stub — mounts <Puck> with the generated config
  // curated companion tables
  SLOTS, OVERRIDES, BINDABLE, PALETTE_TAXONOMY,
  RESOLVER_ONLY_VARIANTS,
} from "@nube/starter-ui-sdui-puck";
```

## Curation surface (`src/curation/`)

Four hand-written tables travel with the generator. They are the
single source of truth for the semantic gaps the JSON Schema does
not carry — see scope §B1 "Generator input = schema + three
curated tables" (this package adds `palette-taxonomy.ts` as the
fourth, per the table in §B1):

| File | Contents |
|---|---|
| `slots.ts` | Layout drop-target tuples (`page.children`, `row.children`, …). |
| `overrides.ts` | Per-variant `ComponentConfig` overrides. PR1 entries are placeholder `null`s for `repeat` / `wizard` / `form` / `table`. |
| `bindable.ts` | Typed leaves that also accept `{{$page.x}}` bindings. PR1 covers `chart.range.{from,to}`, `drawer.open`, `kpi.value`. |
| `palette-taxonomy.ts` | Variant → `"layout" \| "display" \| "interactive" \| "custom"` bucket. |

`overrides.ts` includes a module-load assertion that the
resolver-only variants (`forbidden`, `dangling`, `unknown`) are
never registered.

## Harness

PR1 ships a tiny Vite harness so reviewers can pop the editor open
without wiring it into rubix/frontend:

```bash
pnpm install
pnpm --filter @nube/starter-ui-sdui-puck run harness
# → http://localhost:5180/
```

The harness mounts a `<PuckBuilder>` over a hand-authored four-widget
tree (`heading`, `row`, `kpi`, `chart`) — drag a tile from the
palette to confirm the canvas + palette wire end-to-end. **No save,
no liveness; PR1 is canvas-only.** The last `onChange` payload is
mirrored to `window.__rubixPuckLastChange` for inspection.

## Test / drift guard

`pnpm --filter @nube/starter-ui-sdui-puck test` runs:

1. `scripts/check-schema-drift.mjs` — re-runs
   `cargo run -p starter-ui-ir --bin emit_schema` and fails if the
   committed schema doesn't match. Skips with a warning when
   `cargo` is missing (set `RUBIX_PUCK_DRIFT=strict` to turn that
   into a hard failure on a CI runner that ought to have Rust).
2. The vitest suite — generator coverage, slot vs. array
   assertions, resolver-only exclusion, and a serialisable snapshot
   of the generated config for diff visibility.

## Out of scope for PR1

- **§B3** data-source selectors (templates, tool ids, kinds,
  tenants, units). Every `$ref`-typed leaf currently renders as a
  text field.
- **§B4** save path. `PuckBuilder.onChange` writes the last payload
  to `window.__rubixPuckLastChange` instead of calling
  `rubix.dashboard.update`.
- **§B5** `/dashboards/$pageId/edit` route. Lives in
  `rubix/frontend/` and lands in its own PR.
- **§B6** runtime schema-hash banner. PR1 covers the CI-time drift
  guard only.
- **`PlaceholderRender`** in `@nube/starter-ui-sdui-react`. PR1
  uses a stringify placeholder; the real per-variant placeholders
  ship in scope §B2's own PR.
- Anything in [`rubix/docs/scope/dashboards/11-live-canvas-sse.md`](../../rubix/docs/scope/dashboards/11-live-canvas-sse.md).
