# SDUI components — the widget catalogue and its API surface

> Cites: [`crates/starter-ui-ir/src/component.rs`](../../../../crates/starter-ui-ir/src/component.rs),
> [`crates/starter-ui-ir/schema/starter-ui-ir.schema.json`](../../../../crates/starter-ui-ir/schema/starter-ui-ir.schema.json),
> [`crates/starter-ui-ir/src/bin/emit_schema.rs`](../../../../crates/starter-ui-ir/src/bin/emit_schema.rs).
> Sibling design docs: [`renderer/`](../renderer/README.md),
> [`tools/`](../tools/README.md), [`ai-builder/`](../ai-builder/README.md).
> Active scope referencing this surface:
> [`scope/dashboards/10-puck-builder.md`](../../../scope/dashboards/10-puck-builder.md)
> — visual drag-and-build editor (Puck) over the same IR + the
> three curated companion tables described below;
> [`scope/dashboards/11-live-canvas-sse.md`](../../../scope/dashboards/11-live-canvas-sse.md)
> — live refresh of the read route and editor canvas when the AI
> or another operator writes a new revision (extends the existing
> sidebar SSE channel).

This document answers two questions that come up every time a new
authoring surface is proposed (AI flow, Puck visual builder,
extension UI panels):

1. **Where does the list of available widgets live?**
2. **How does an authoring client discover that list?**

The short answer: **the backend defines the palette structurally;
clients ship it at build time. There is no runtime widget-catalogue
verb today.** This doc records the chain end-to-end so future
authoring paths don't reinvent it.

## Source of truth — the `Component` enum

Every SDUI widget is one arm of the
[`Component` enum](../../../../crates/starter-ui-ir/src/component.rs)
in `starter-ui-ir`. Adding a widget = adding one variant + one
renderer per platform. There is no second registry, no plugin
table, no per-tenant overlay.

The enum groups today's variants into four working categories
(used by the AI builder's intent-table and by the Puck palette's
taxonomy map):

| Category | Variants |
|---|---|
| **Layout** | `Page`, `Row`, `Col`, `Grid`, `Tabs`, `Section`, `Divider`, `Repeat`, `Drawer`, `Dialog`, `Wizard`, `Card`, `FieldGroup` |
| **Display** | `Text`, `Heading`, `Badge`, `Markdown`, `RichText`, `Kpi`, `KpiGrid`, `Chart`, `Sparkline`, `Table`, `ArrayTable`, `JsonTable`, `List`, `Tree`, `Timeline`, `Detail`, `Diff`, `Menu`, `ActionWidget` |
| **Input (write-path)** | `Form`, `TextField`, `NumberField`, `Textarea`, `Toggle`, `Slider`, `Checkbox`, `Select`, `SelectField`, `RadioGroup`, `Segmented`, `DateField`, `DateRange`, `RefPicker`, `MarkdownEditor`, `Button` |
| **Resolver-only (not author-time)** | `Forbidden` (ACL-redacted), `Dangling` (unsatisfiable / forward-compat downgrade), `Custom` (extension-registered renderer) |

The resolver-only category is important for any authoring client:
**these variants must not appear in a palette.** They are emitted by
the resolver on read, never authored. A palette generator that
walks the schema naively will surface them as drag tiles unless it
is told otherwise — see "Curated companion tables" below.

## Emission — JSON Schema as the wire contract

The Rust enum is emitted as JSON Schema by:

```bash
cargo run -p starter-ui-ir --bin emit_schema
```

Output: [`crates/starter-ui-ir/schema/starter-ui-ir.schema.json`](../../../../crates/starter-ui-ir/schema/starter-ui-ir.schema.json)
(~4,600 lines). The artifact is **committed**, not generated at
build time, so:

- TS/Dart clients import it directly from the path above.
- A CI guard ([`schema_artifact.rs`](../../../../crates/starter-ui-ir/tests/schema_artifact.rs))
  re-runs `emit_schema` and diffs the result; any Rust change that
  shifts the schema without a matching JSON commit fails CI.

The schema is the **shape contract**. What it does *not* encode:

- Which `children: [Component]` arrays are **drop-target slots**
  (layout containers) vs **authored arrays** (e.g. `kpi_grid.kpis`,
  `chart.sources`). Schemars emits both as `array<Component>`.
- Which leaf fields accept a **binding expression** (`{{$page.x}}`,
  `{{$user.y}}`) in addition to their typed value.
- Which variants are **author-time** vs **resolver-only**.
- Per-variant **placeholder rendering** for build-time palettes.

These are out-of-band facts. Any authoring client needs them in
addition to the schema — see the next section.

## Curated companion tables

Three small tables travel with the schema. They are *not* in the
schema today, by deliberate choice: schemars-generated output is
fragile enough that overloading it with semantics would break the
CI diff check.

| Table | Lives | Used by |
|---|---|---|
| **Slot variants** — which `children` arrays are layout drop-targets | Authoring-client side (Puck config generator, AI builder skill prompt) | Palette generator to decide slot vs array field |
| **Authored variants** — the subset of `Component` an author may emit (excludes `Forbidden`, `Dangling`, and most `Custom` paths until registered) | Same | Palette population, AI skill `allowed_tools` filtering |
| **Binding-eligible fields** — typed leaves that also accept binding syntax | Same | The `<BindingAwareField>` wrapper in any visual editor |

Each authoring surface that has appeared so far (the AI builder, the
proposed Puck visual builder) re-derives these tables. **For v1
that's acceptable; for v2 we consolidate them into one Rust-side
declaration with derived JSON output** — see "Future work" below.

## API surface today — what exists and what doesn't

### What exists

| Verb / endpoint | Role |
|---|---|
| `rubix.dashboard.get` / `.create` / `.update` / `.list` / `.duplicate` / `.delete` / `.page_set` | The seven dashboard verbs ([`tools/`](../tools/README.md)). They accept and emit `ComponentTree` bodies; the server validates against the compiled-in schema. |
| `GET /api/v1/ui/resolve` | Resolver endpoint — turns an authored tree + a data context into a fully-resolved tree the renderer walks. |
| `GET /api/v1/tools` | Lists registered tool verbs. Used by action-target pickers to validate `Action.target.tool` strings. **Not** a widget list. |
| `crates/starter-ui-ir/schema/*.schema.json` (committed artifact) | The widget list, structurally — but consumed at client **build time**, not at runtime. |

### What doesn't exist (yet)

| Missing verb / endpoint | Why it would help |
|---|---|
| `GET /api/v1/ui/schema` | The compiled-in schema body. Today every authoring client snapshots the artifact at build time and is blind to backend schema drift after deploy. |
| `GET /api/v1/ui/schema/hash` | Just the schema's hash. Cheaper than serving the full body; enables a mismatch banner without 30 KB on every editor mount. Currently proposed in scope [`10-puck-builder.md` §B6](../../scope/dashboards/10-puck-builder.md#b6-schema-drift-guard). |
| `rubix.ui.list_widgets` (or `GET /api/v1/ui/widgets`) | A flat, taxonomy-grouped catalogue of *authored* variants with placeholder hints and binding-eligibility flags. The natural home for the three curated companion tables above so each authoring client stops re-deriving them. |
| `rubix.ui.list_chart_sources` | Today the AI builder and the Puck builder both need to know what `ChartSource` variants are renderable. Same situation as widgets — structurally in the schema, but operationally hand-maintained per client. |

## Drift model — where the system can go wrong

Today's setup catches one drift mode and misses two:

- ✅ **CI-time drift** between Rust IR and committed schema —
  caught by [`schema_artifact.rs`](../../../../crates/starter-ui-ir/tests/schema_artifact.rs).
- ❌ **Build-vs-deploy skew** — a frontend bundle pins whatever
  schema was on disk at build time. After a backend upgrade adds
  a variant, the editor silently lacks the new tile until the
  frontend is rebuilt.
- ❌ **Multi-client divergence** — the React renderer, the Flutter
  renderer, and any future Puck editor each ship their own snapshot
  of the schema. Nothing forces them to be the same version at
  runtime; the wire is forward-compat via `Unknown` fallbacks but a
  shape change can silently produce `Dangling` widgets on older
  clients.

The proposed `GET /api/v1/ui/schema` (or its hash variant) closes
build-vs-deploy skew. Multi-client divergence is intrinsic to
shipping native renderers and is handled by the resolver's V3→V2
downgrade path ([`renderer/`](../renderer/README.md)) rather than
by the catalogue itself.

## How the existing authoring paths use the catalogue

### AI builder (`com.rubix.dashboard-assistant`)

The skill body in
[`rubix-skills/skills/dashboard-builder/SKILL.md`](../../../crates/rubix-skills/skills/dashboard-builder/SKILL.md)
hand-lists the variants the model is expected to emit, with a
defaults table mapping operator intent → starter layout. The
**model never reads the schema** — it relies on the prose plus the
`allowed_tools` filter. Drift between the prose and the IR is
caught only at `rubix.dashboard.update` time, when the host
deserialises the tree.

This is acceptable while the variant set is small; it gets brittle
once new layout/display variants land. Long-term, the skill should
reference a server-served variant catalogue rather than
re-enumerating it in prose.

### Puck visual builder (proposed,
[`scope/dashboards/10-puck-builder.md`](../../scope/dashboards/10-puck-builder.md))

`@nube/starter-ui-sdui-puck` imports the committed JSON Schema at
Vite build time, walks the `Component.oneOf`, and emits one Puck
`ComponentConfig` per variant. The three curated companion tables
ride alongside the generator. B6 of that scope proposes a
schema-hash drift banner so the operator at least knows when the
editor is older than the server.

### React renderer (`@nube/starter-ui-sdui-react`)

The renderer's per-variant `render-*.tsx` files match the IR by
convention; a missing renderer for a known variant produces a
visible `Dangling` placeholder. No schema fetch; the renderer is
shape-shaped to the *frozen* IR version it was built against.

## Future work — consolidating the catalogue

The direction is **make the backend the runtime source of truth
for both shape and semantics, not just shape.** Concretely:

1. **One Rust-side declaration of widget metadata** — move the
   three curated companion tables into a `WidgetCatalogue` struct
   in `starter-ui-ir` (or a thin wrapper crate). Each `Component`
   variant gets a derive-or-inventory entry with: category, slot
   children, authored-array children, binding-eligible field
   paths, placeholder hint, author-time vs resolver-only flag.
2. **Emit alongside the schema** —
   `crates/starter-ui-ir/schema/widget-catalogue.json` becomes a
   second committed artifact, regenerated by `emit_schema`. CI
   guard extends to diff both.
3. **Serve it** — register `rubix.ui.list_widgets`
   (MCP-and-REST, per the
   [MCP-only-for-AI direction](../../sessions/data-flow/07-ai-authoring.md))
   returning the catalogue + a schema hash. Authoring clients
   prefer the runtime fetch; the build-time snapshot becomes the
   fallback.
4. **Reference it from the AI skill** — the dashboard-builder
   skill body cites the catalogue by name rather than re-listing
   variants. The model can read it the same way it reads any
   other resource.

None of this is urgent — the v1 path works. The cost it imposes
is on every *new* authoring surface, each of which today must
re-derive the same three tables. When the second one lands
(Puck), it is the right time to extract.

## Quick decision guide

When a new authoring path needs a widget palette:

- **It needs a list of variants and their shapes** → import the
  JSON Schema artifact at build time. This is correct for v1 and
  matches what every existing client does.
- **It needs to know "is this variant author-time?"** → curate
  that list in the client today; flag it as a candidate for the
  v2 `WidgetCatalogue` consolidation.
- **It needs to track backend variants added after the frontend
  shipped** → wait for `GET /api/v1/ui/schema` (or its hash). Do
  not invent a per-surface fetch path.
- **It needs to validate a tree before submitting** → don't
  re-implement the schema check; submit and let
  `rubix.dashboard.update` return the typed error. The server is
  the authority.
