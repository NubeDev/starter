# 10 — Puck visual builder

> **Tier:** scope (plan). Lifetime: weeks. Per
> [HOW-TO-CODE.md §0a](../../../../HOW-TO-CODE.md), **no source
> code may reference this file.** Promote landed sections into
> `docs/design/sdui/builder/README.md` once shipped.

## Goal

Give operators a **visual drag-and-build editor** for SDUI pages
that writes the same `ComponentTree` JSON the AI assistant emits.
The editor is a sibling to the AI builder, not a replacement:
both paths produce identical bodies, persisted through the same
`rubix.dashboard.update` verb, rendered by the same
`@nube/starter-ui-sdui-react` renderers.

The editor library is **[Puck](https://github.com/puckeditor/puck)**.
This file scopes only the integration — Puck's own docs are the
source of truth for its config API.

## Why Puck (vs alternatives)

Decided 2026-05-26 after a quick survey:

| Option | Native data model | Fit for our IR | Verdict |
|---|---|---|---|
| **Puck** | JSON tree `{type, props, children}` | 1-to-1 with `Component { type, … }` | ✅ chosen |
| GrapesJS | DOM + CSS rules | Requires HTML↔IR reverse-parse on every save | ❌ wrong shape |
| Craft.js | Internal node store | Adapter layer needed; opinionated editor state | ❌ heavier wrapper |
| react-grid-layout + dnd-kit | None — you build the editor | Most control, most code | Deferred — revisit if Puck hits a wall |

Puck owns the canvas, the palette, the props inspector, and the
undo stack. We own the IR and the persistence path. The
integration layer is a single `Config` object that maps each
IR `type` enum value to a Puck component definition.

## User story

> *Operator opens `/dashboards/data-flow-site-a/edit`. They see
> the current page on the left, a palette of available widgets
> on the right, and a props panel below. They drag a `chart`
> tile into a `row → col`, pick a template + meter_id from
> dropdowns, type a title, and hit **Save**. The body is
> persisted via `rubix.dashboard.update`; the read-only route
> at `/dashboards/data-flow-site-a` reflects it on next resolve.*

The same route can be opened by the AI assistant flow's
`rubix.dashboard.update` call. Operators see what the AI built
without leaving the same UI surface.

## Non-goals (in this scope)

- **No new IR.** Every drop produces a `Component` literal that
  the existing Rust deserialiser accepts unchanged.
- **No HTML / CSS authoring.** Pixel layout comes from `row` /
  `col` / `grid` IR variants. Puck's CSS escape hatch is not
  exposed.
- **No live preview against real data.** The canvas renders
  widgets in **placeholder mode** — fake series, fake KPI
  values. Real resolve happens at `/dashboards/<slug>` (read
  path) as today. (Stretch goal: a "Preview" tab inside the
  editor that calls `/ui/resolve` against the in-memory tree.)
- **No multi-user concurrent editing.** Optimistic concurrency
  via `expected_revision_id` (already in
  [`rubix.dashboard.update`](../../design/sdui/tools/README.md));
  a conflict prompts a re-fetch + rebase by the operator.
- **No bespoke widget marketplace.** The palette is exactly the
  IR's `Component` variants. Extensions ride the existing
  `Component::Custom` path; that bridge is a separate scope.
- **No mobile / Tauri editor.** v1 is web-only. The *output* is
  still cross-platform because nothing about the body changes.
- **Mouse-first authoring on the canvas.** Puck is drag-and-
  drop; the canvas inherits that. Keyboard authoring of the
  canvas itself (full WCAG 2.1.1 reachability of palette + tree
  via Tab/Enter/Arrow) is **deferred**. The complementary
  keyboard-friendly authoring path is the **operator chat
  surface** that drives `com.rubix.dashboard-assistant` — typing
  natural language at the AI is a first-class authoring mode,
  not a fallback, and Puck is the visual-confirmation layer
  over what the AI (or operator) wrote. Read-side accessibility
  of rendered pages is unchanged — that lives in
  `@nube/starter-ui-sdui-react`.
- **No role-impersonation preview.** SDUI bodies can contain
  action targets that the editing operator can invoke but a
  viewer cannot (or vice versa); resolve-time authz filters
  these on the read route. The editor canvas always renders
  from the operator's perspective and may therefore show more
  (or fewer) interactive widgets than the read route will when
  a viewer opens the page. "Preview as <role>" is deferred.

## What we need to build

### B1. New frontend package: `@nube/starter-ui-sdui-puck`

Sibling to `@nube/starter-ui-sdui-react`. Exports:

```ts
export function buildPuckConfig(opts: {
  rendererCatalog: Record<string, ReactRenderer>; // from sdui-react
  irSchema: object;                               // imported JSON Schema
}): import("@measured/puck").Config;

export function PuckBuilder(props: {
  pageRef: string;            // "dashboard.<slug>"
  initialTree: ComponentTree;
  onSave(tree: ComponentTree, expectedRevisionId: string): Promise<void>;
}): JSX.Element;
```

`buildPuckConfig` walks the IR JSON Schema's `Component` `oneOf`
arm-by-arm and emits one Puck `ComponentConfig` per variant.
Field mappings are driven by JSON-Schema types:

| IR property shape | Puck field |
|---|---|
| `"type": "string"` | `text` |
| `"type": "string", "enum": [...]` | `select` with the enum |
| `"type": "number"` | `number` |
| `"type": "boolean"` | `radio` (yes/no) |
| `"type": "array", "items": {…Component…}` on layout variants (`row.children`, `col.children`, `page.children`, `grid.children`, `card.children`, `tabs.tabs[].children`) | Puck **slot** (drag-target on the canvas), not an array field |
| `"type": "array", "items": {…Component…}` on data-bearing variants (e.g. `chart.sources`, `kpi_grid.kpis`) | `array` of nested object fields — these are authored, not drop-targeted |
| `"type": "array", "items": {…primitive…}` | `array` of scalar fields |
| `"$ref": "#/definitions/<X>"` (non-Component) | `external` or `custom` field — selector tool, see B3 |
| Any typed leaf the IR also accepts a binding for (`{{$page.foo}}` etc.) | Field wrapped by `<BindingAwareField>` — toggles between the typed editor and a binding picker (see Q3) |

Variants whose shape doesn't fit (e.g. `Repeat`, `Wizard.steps`)
get a hand-written `ComponentConfig` override registered after the
auto-generation pass. Override registry lives next to the
generator so the override surface is greppable.

**Schemars output is not a clean discriminated union.** The IR
emits enum-tagged variants under `Component.oneOf`, but several
shapes need explicit handling rather than naive traversal:

- `$ref`-heavy types (`ChartSource`, `Action`, `NodeStyle`) —
  the generator must follow the ref and treat the target as a
  nested object field, not paste the `$ref` literal.
- `oneOf` nested inside `allOf` (the schema's discriminator
  wrapping pattern) — flatten before mapping.
- `style: NodeStyle` on every layout variant — **skipped** in
  v1 per Q2. The skip lives in the generator (one allow-list of
  property keys to drop), not at field-render time.
- Binding-string-or-typed-leaf — every typed leaf that the IR's
  binding grammar accepts (`{{$page.x}}`, `{{$user.y}}`, etc.)
  must route through `<BindingAwareField>` (above). The list of
  binding-eligible fields is **not** in the schema; it has to be
  curated alongside the generator. v1 covers `$page` only, so
  the curated list is short.

If Q3 lands "bindings work everywhere," B1's binding-aware
wrapper is the carrier. If Q3 lands "no bindings in v1," the
wrapper is omitted and the matrix row above drops out — the two
items move together.

**Generator input = schema + three curated tables.** The JSON
Schema alone is *not* sufficient. To keep the curation surface
discoverable instead of scattered across the codebase, the
generator reads exactly four files, all colocated in
`@nube/starter-ui-sdui-puck/src/curation/`:

| File | Contents |
|---|---|
| `slots.ts` | Tuples `(variantType, propertyName)` whose children-array is a Puck **slot** (drop-target), not an array field. Concrete v1 list: `(page, children)`, `(row, children)`, `(col, children)`, `(grid, children)`, `(card, children)`, `(section, children)`, `(tabs.tabs[], children)`. |
| `overrides.ts` | Map `{ variantType → ComponentConfig }` for variants whose shape the auto-generator cannot handle cleanly. v1 list: `Repeat`, `Wizard`, `Form`, `Table` (toolbar/row actions). |
| `bindable.ts` | Set of `(variantType, propertyName)` tuples whose typed leaf also accepts a `{{$page.x}}` binding string. v1 covers `$page` only, so the list is short — start with `range.from/to` on `Chart`, `open` on `Drawer`, KPI `value`. |
| `palette-taxonomy.ts` | Map `{ variantType → "layout" \| "data" \| "interactive" \| "display" \| "custom" }`. Variants absent from the map show as "uncategorised" (visible breakage). |

The generator is a pure function of `(schema, slots, overrides,
bindable, taxonomy) → Puck.Config`. New contributor sees four
files, not a stack trace.

### B2. Palette derived from the IR schema

The palette is **not** hand-curated. It enumerates the
`Component` `oneOf` variants and groups them by the
`palette-taxonomy.ts` map from B1. New variants without a
classification show as "uncategorised" — visible breakage, not
silent drift.

**Author-time vs resolver-only variants.** `Component::Forbidden`,
`Component::Dangling`, and `Component::Unknown` are emitted by
the server's resolve / capability-handshake path — they are not
author-time tiles. The palette explicitly excludes them. The
generator also rejects any attempt to register a `ComponentConfig`
for these names in `overrides.ts` (assertion at build time) so
the exclusion can't be accidentally re-introduced.

Each palette tile renders its widget in **placeholder mode**:

- `kpi` → "123.4 kWh" (static).
- `chart` → a 6-point sine wave.
- `table` → 3 fake rows.
- `kpi_grid`, `repeat` → 3 placeholder children.

Placeholder mode is **new work**, not a re-export. The
sdui-react package today has per-variant placeholders inside
individual `render-*.tsx` files (see `render-custom.tsx` for the
pattern); `test-utils.tsx` only exports `nullTransport` and
`renderHarness`. B2 includes building a unified
`<PlaceholderRender node={…} />` in `@nube/starter-ui-sdui-react`
that dispatches to the same per-variant placeholders the live
renderer uses when its transport returns empty, plus extending
the placeholder coverage to every variant that lacks one today
(at least `kpi`, `chart`, `table`, `kpi_grid`, `repeat`,
`form`). The puck package re-exports it from there so canvas and
runtime stay aligned.

### B3. Data-source selectors (the only non-generic bit)

Some IR fields point at server-side resources, not free text:

| IR field | Selector |
|---|---|
| `source.name` on `analytics_template` | Dropdown populated from `GET /api/v1/tools/rubix.analytics.list_templates` (new — see follow-up). |
| `source.params.tenant_id` | Text field — autocompletes from `auth.me.tenant_id` and tenants the operator can read. |
| `kpi.unit_symbol` | Free text with common-unit suggestions (kWh, L, °C, %). **Display-only string** — the data path's `Quantity` unit is authoritative for aggregation. The picker labels the field accordingly so operators don't expect `"kWh"` vs `"kwh"` to merge. (Closed enum from a unit catalogue is a v1.5 hardening.) |
| `table.source.kind` (RSQL) | Dropdown from the host-glue kind catalogue. |
| Action targets (`navigate`, `tool_call`) | Two-step picker: kind → id. See validation note below. |

These selectors are **Puck custom fields** that call rubix tool
verbs. Each one is one ~50-line React component; collectively
they're the bulk of the "build" surface.

**Fetch lifecycle.** All selectors fire their lookup on editor
mount in parallel (templates, tools, kinds, tenants, units),
cache for the editor session in a single React context
(`<EditorCatalogProvider>`), and on individual failure the
affected picker **degrades to free-text with an inline warning**
(`"couldn't load template list — typing the name still works"`).
The Save path's catalogue checks (action targets, below) re-fetch
on save so a stale cache can't silently approve a removed
target.

**No live picker refresh in v1.** If a new template / tool /
action target is registered after editor mount but before save,
the picker won't surface it until the operator reloads. Save-
time re-fetch covers correctness; live picker refresh (an SSE-
driven `useToolCatalogLiveness` mirroring the dashboard channel)
is filed as a v1.5 hardening, not a bug.

**Action target validation — save-time, not resolve-time.** A
typo in an `Action.target.tool` is the highest-impact failure
mode (silent dead click in production). The action picker
populates from `GET /api/v1/tools` at editor open; the picker
itself prevents free-form typing. On save, `PuckBuilder` walks
the tree, collects every action target, and verifies each
against the same catalogue before calling
`rubix.dashboard.update`. Mismatches block the save with an
inline error pointing at the offending node, instead of letting
the operator find out at resolve time. Resolve-time authz
filtering (a viewer cannot invoke a target the operator could)
is unchanged — that's a separate concern, see the new non-goal
below.

### B4. Save path

`PuckBuilder.onSave` calls `rubix.dashboard.update` via
the standard REST envelope (cookie + CSRF). The request shape
is owned by
[`UpdateDashboardRequest`](../../../crates/rubix-spi/src/dto/dashboard/update.rs)
(`tenant_id`, `page_id`, `expected_revision_id?`, `title?`,
`tags?`, `body_json`, `created_by`) — the implementer must read
the DTO at the time of building, not the snapshot below. The
example here is illustrative only:

```json5
// Illustrative — verify against UpdateDashboardRequest before use.
{
  "tenant_id":            "<from loader>",
  "page_id":              "dashboard.<slug>",
  "expected_revision_id": "<from loader>",
  "title":                "<from Puck root>",
  "body_json":            { /* ComponentTree */ },
  "created_by":           "<auth.me.principal>"
}
```

On HTTP 409 (`rubix.dashboard.update.conflict`) the editor shows
a "this page was edited elsewhere" modal with two options:

- **Discard my edits** — reload the server's revision and lose
  the in-editor tree.
- **Keep editing** — stay on the in-editor tree, see a persistent
  warning banner, and accept that the next Save will 409 again
  until the operator either Discards or copies their edits out
  by hand.

There is no "Merge manually" path — three-way diffing of a
`ComponentTree` JSON in a modal is not realistic UX for v1.
Operators with valuable in-flight edits paste their work into a
gist before Discard, the same way the flow-programmer handles
its own lint conflicts today.

**The 409 modal is the fallback, not the primary signal.** Scope
[11 §B3](./11-live-canvas-sse.md) fires a non-blocking
"AI/operator updated this page just now" banner the moment a
divergent revision lands (~1 s after commit), well before the
operator hits Save. The 409 modal here handles only the case
where the operator dismissed or never saw the banner.

### B5. Route

New route `/dashboards/$pageId/edit` in
[`rubix/frontend/src/routes/dashboards/`](../../../frontend/src/routes/dashboards/).
The route loader fetches the live revision
(`rubix.dashboard.get`) and hands the body + `revision_id` to
`<PuckBuilder>`.

**File-routing restructure.** The current read route is the
flat file `dashboards/$pageId.tsx`. Adding an `/edit` child
under TanStack file-based routing requires either:
- converting to a folder — `dashboards/$pageId/index.tsx` (the
  current body, moved) + `dashboards/$pageId/edit.tsx`, or
- using flat dotted routing — `dashboards/$pageId.edit.tsx`
  alongside the existing file.

Folder-form is the cleaner refactor and is what this scope
assumes. Either way, regenerate `routeTree.gen.ts`.

**Edit-permission wiring.** `rubix.dashboard.update` already
declares
[`REQUIRED_PERMISSION = "rubix.dashboard.edit"`](../../../crates/rubix-spi/src/dto/dashboard/update.rs),
so server-side gating exists today via the verb. What's missing
for B5:

- `authz.rs` registers only the `rubix.dashboard.page` resource
  *kind*, not the individual `rubix.dashboard.edit` *permission
  string* as something the frontend can query before showing the
  Edit button.
- **v1 decision: dead-button path.** The read-only route always
  shows the Edit button. Clicking it opens the editor; Save
  fails with 403 if the principal lacks `rubix.dashboard.edit`,
  and the editor surfaces the failure as a permission error.
  Matches how every other write verb is gated in the frontend
  today; avoids adding a new capability-handshake field for one
  button.
- **Not v1: capability-driven hide.** Exposing the verb's
  required permission so the button can hide pre-emptively is
  deferred. Revisit only if user research shows the dead-button
  click is misleading enough to matter — until then, "click,
  see error" is acceptable and consistent.
- **Not v1: AI-in-flight prompt at editor open.** Once scope
  [11](./11-live-canvas-sse.md) §B7 lands (in-flight AI
  events), the editor route can call `usePageLiveness` at mount
  and prompt *"AI is editing this page right now — open
  anyway?"* before the operator commits to an edit session.
  Until then the operator opens blind and learns about the AI's
  activity via the post-commit banner.

### B6. Schema-drift guard

The IR schema lives in
[`crates/starter-ui-ir/schema/starter-ui-ir.schema.json`](../../../../crates/starter-ui-ir/schema/starter-ui-ir.schema.json)
and is regenerated by `cargo run -p starter-ui-ir --bin emit_schema`.
The puck package imports it at **build time** (vite resolves the
JSON), so a CI step must:

1. Run `emit_schema` and diff against the committed JSON —
   existing check, see
   [`crates/starter-ui-ir/tests/schema_artifact.rs`](../../../../crates/starter-ui-ir/tests/schema_artifact.rs).
2. Re-run the puck package's typecheck against the regenerated
   schema (new — add to `pnpm test`).

Drift between the Rust IR and the puck palette becomes a CI
failure, not a runtime mystery.

**Runtime drift (build-vs-server skew).** The puck bundle is
pinned to whatever schema was on disk at build time. If the
running server has been upgraded to a newer IR (a variant added,
a field added) the editor will silently miss it — saving a body
without the new field is fine, but the operator never gets the
new tile. CI drift only catches CI-time skew, not deploy-time
skew. B6 also adds:

- A `GET /api/v1/ui/schema` endpoint (or extend the existing
  capability handshake response) that returns the *hash* of the
  schema the server compiled in.
- The puck bundle embeds the same hash from its build-time
  schema import.
- On editor mount, compare hashes. Mismatch shows a non-blocking
  banner: *"Editor is built against an older schema (`abc1234`);
  the server is on `def5678`. Some widgets may be missing —
  reload to pick up the latest."*

Mismatch is **non-blocking only because IR changes are
additive**. The promotion rule for the IR (see
[`docs/design/sdui/README.md`](../../design/sdui/README.md)) is
that variants and fields may be *added*, but existing variants
keep their required-field set forever (new fields land
`Option<>` first, are promoted later only with a wire-compat
bump). As long as that holds, a build-time-frozen editor can
always serialise a body the server will accept — it just won't
know about new tiles. If the IR ever ships a breaking change
(new required field on an existing variant, renamed type tag),
the banner becomes **blocking** for that variant until the
editor is rebuilt: `schema_artifact.rs` gains a "breaking-
change" marker the server publishes alongside the hash, and the
editor disables the affected tiles on mismatch.

**Considered and deferred: runtime schema hot-load.** Going one
step further — exposing `GET /api/v1/ui/schema` returning the
full schema and hot-loading it into the puck config on mount —
would make the backend the single runtime source of truth and
collapse build-time skew into a startup check. The cost is
real:

- Puck's `Config` includes React component refs (live
  renderers), not just data. A schema delivered from the server
  cannot bring its own renderers; the build-time generator has
  to still run on the *static* renderer catalogue, then patch in
  the runtime schema's variant set. New variants whose
  renderers the bundle doesn't ship show as the existing
  `Component::Unknown` placeholder.
- Adds a new public endpoint and a ~30 KB cold-mount payload
  whose schema-hash dance still ships in v1 above.
- v1 ships are CI-gated and deployed together, so build-time
  drift is rare. The benefit lands when build and deploy
  decouple (extensions ship their own variants, multi-cluster
  staggered rollouts), neither of which is v1 territory.

Decision: keep the build-time import + hash check as v1.
Runtime hot-load is filed as Q5 below for revisit when
extension-shipped variants land.

## Dependency order

```
B1 (config generator)  ──┐
                          ├──►  B4 (save path)  ──►  B5 (route)
B2 (palette taxonomy)  ──┤
                          │
B3 (data-source pickers) ─┘                    ┌── B6 (drift CI)
                                               │
                                               └── ships alongside B1
```

B1 + B2 + B3 are each one focused PR. B4 is trivial once they
exist (it's just `fetch` + concurrency handling). B5 wires the
route. B6 is a CI-only PR.

## Open questions

| # | Question | Default if no one answers |
|---|---|---|
| Q1 | Should the editor write directly to `dashboards_definitions` (current `rubix.dashboard.update`) or stage drafts in a separate table? | Direct write. Concurrency token + history table (already present) covers the rollback story. Drafts revisited if operators ask. |
| Q2 | Do we expose `style: NodeStyle` (color, padding) in the props panel? | **No** in v1. Theme tokens are the only knob. Avoids the "every page looks different" failure mode. |
| Q3 | How do we surface `bindings` (`{{$page.foo}}` etc.) in the props panel? | A "bind to…" toggle on each field that swaps the input for a binding picker (page-state key dropdown). v1 covers `$page` only; `$user`, `$stack` deferred. |
| Q4 | Does the AI assistant emit a *diff* that the editor previews before save, or does it commit and rely on revision history for undo? | **Resolved in scope [11-live-canvas-sse.md](./11-live-canvas-sse.md).** Commit + history is the write model; scope 11's `usePageLiveness` + canvas banner is the *post*-commit visibility path. Pre-commit / in-flight AI visibility is deferred (see scope 11 §B7). |
| Q5 | Runtime schema hot-load from `GET /api/v1/ui/schema`. | **Deferred.** Build-time import + hash banner ships in v1; runtime hot-load revisited when extension-shipped variants land. See B6 above for the trade-off analysis. |
| Q6 | Live concurrency with the AI assistant — operator opens the editor while the AI is mid-stream updating the same page. | **Resolved in scope [11-live-canvas-sse.md](./11-live-canvas-sse.md).** The editor subscribes to the existing `/api/v1/dashboards/events` SSE channel via a new `usePageLiveness(pageRef)` hook and shows a non-blocking "AI updated this page — Reload / Keep editing" banner on a divergent revision. 409-on-save remains the ultimate safety net for operators who dismiss the banner. |

> The `analytics.list_templates` verb that B3 depends on is a
> tools-domain decision and belongs in
> [`04-tools.md`](./04-tools.md), not as an open question on
> this scope. Listed here only so the dependency is visible.

## How this maps to SCOPE.md

- **Goal 1** — extends the existing dashboard surface with a
  second authoring path (visual + AI). No new goal added.
- **R12** — MCP resource URIs are unaffected; the editor is a
  web-only surface that calls existing REST verbs.

## A note on cited line numbers

Same disclaimer as the parent README — line numbers in scope
files are anchors for the implementer to find the right symbol,
not stable references. Re-grep before quoting.
