# @nube/starter-ui-sdui-puck

Visual SDUI editor powered by [Puck](https://github.com/puckeditor/puck).
Generates a Puck `Config` from the committed
[`starter-ui-ir` JSON Schema](../../crates/starter-ui-ir/schema/starter-ui-ir.schema.json)
and renders a canvas + palette over the same widget vocabulary the
AI builder emits and `@nube/starter-ui-sdui-react` renders.

This package implements the Puck builder per
[`rubix/docs/scope/dashboards/10-puck-builder.md`](../../rubix/docs/scope/dashboards/10-puck-builder.md).

## Status (2026-05-26)

| Scope | Landed | Notes |
|---|---|---|
| §B1 schema → PuckConfig | ✅ | `buildPuckConfig` + curated tables |
| §B2 PlaceholderRender + palette | ✅ | per-variant placeholders, taxonomy buckets |
| §B4 save seam (Save button, 409 modal) | ✅ | `save.ts` + `PuckBuilder` toolbar |
| §B5 `/dashboards/$pageId/edit` route | ✅ | lives in `rubix/frontend`, uses `$pageId_.edit.tsx` (non-nested) so it doesn't render inside the read route's layout |
| §B6 CI drift guard | ✅ | `scripts/check-schema-drift.mjs` |
| §B3 data-source selectors | ⏳ | next up — `$ref` leaves still render as text fields |
| §B6 runtime schema-hash banner | ⏳ | CI-time guard only so far |
| Scope 11 (live-canvas SSE) | ⏳ | unstarted |

### Notable infra changes

- **Puck `^0.19.0`** (was `^0.18.0`). 0.18 silently dropped the
  `slot` field type the generator emits, so layout containers
  never became drop zones and the canvas rendered blank. 0.19
  passes slot props to render functions as React components, so
  `placeholder-renderer.tsx` now splits Puck-supplied slot
  components out of the IR-shaped node before delegating to
  `PlaceholderRender`.
- **`makeRubixSaveTransport(client, tenantId)`** — the Rust DTO
  requires `tenant_id` on both `rubix.dashboard.get` and
  `rubix.dashboard.update`; bundled pages use `"system"`. The
  stale TS client (`rubix/packages/rubix-client-ts`) was patched
  to add the field and to flatten `DashboardGetResponse` (the
  response is not nested under `.page`).

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

## Next tasks

1. **§B3 data-source selectors.** Replace the text-field fallback
   for `$ref`-typed leaves (`AnalyticsTemplateRef`, `ToolRef`,
   `TenantId`, unit symbols) with `select`/`autocomplete` fields
   backed by `/api/v1/tools` and the analytics catalogue.
2. **Multi-tenant.** Hardcoded `"system"` tenant in
   `$pageId_.edit.tsx` + `useDashboardGet` needs to come from the
   authed session once tenant scoping lands.
3. **Discard bridge cleanup.** Edit route polls
   `window.__rubixPuckDiscardRequested` every 250ms; replace with
   a `useImperativeHandle` ref or callback prop on `PuckBuilder`.
4. **Placeholder coverage.** Variants without per-variant fillers
   fall through to the dangling tile. Add entries to
   `@nube/starter-ui-sdui-react/src/headless/placeholder-render.tsx`.
5. **§B6 runtime schema-hash banner.** CI-time drift guard only.
6. **Pre-existing test failure** (not blocking the editor):
   `packages/starter-ui-sdui-react/src/renderer/__tests__/render-chart.test.tsx`
   asserts a stale `"3 series"` string.
7. **Scope 11** — live-canvas SSE
   ([`rubix/docs/scope/dashboards/11-live-canvas-sse.md`](../../rubix/docs/scope/dashboards/11-live-canvas-sse.md)).
