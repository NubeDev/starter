# 2026-05-25 — SDUI builder tool gap: what the AI needs to build real BI dashboards

## TL;DR

Chat *can* create dashboards now (see
[`2026-05-25-dashboards-sidebar-sse-and-chat-gaps.md`](2026-05-25-dashboards-sidebar-sse-and-chat-gaps.md)),
but the only tool it has on the dashboards surface is "write a `jsonb` blob
and hope". The first chat-built `dashboard.iot-overview` 404'd at render
time because the LLM invented a `{"layout": ...}` shape that the
`ComponentTree` schema does not accept. The store's column is `jsonb`, so
the bad body sailed through `rubix.dashboard.create` and only blew up
when the SDUI resolver tried to deserialise it.

We just landed the obvious server-side guard (validate `body_json`
against `starter_ui_ir::ComponentTree` inside `create`; same gate going
on `update`). That stops the silent corruption, but it does **not**
help the agent succeed on the next attempt — it just turns a 404 into
an `Invalid` toolcall.

For "powerful BI dashboards" we need the agent to:

1. **Discover** what components, charts, sources, and bindings are
   available.
2. **Validate** a draft tree without persisting it.
3. **Preview** what the tree will render.
4. **Dry-run** the data side (does this RSQL actually return rows?).
5. **Clone / diff** against an existing working dashboard.

None of (1)–(5) exist as tools yet. This session is the gap list.

## What exists today

### IR (the schema)

- `crates/starter-ui-ir/`, `IR_VERSION = 5`.
- ~35 component variants (`page`, `row`, `col`, `grid`, `tabs`,
  `section`, `card`, `kpi`, `kpi_grid`, `chart`, `sparkline`, `table`,
  `array_table`, `json_table`, `list`, `detail`, `timeline`,
  `markdown`, `tree`, `button`, `dialog`, `menu`, `wizard`, `drawer`,
  `form`, `field_group`, `toggle`, `slider`, `text_field`, `select`,
  `date_range`, `ref_picker`, `rich_text`, `markdown_editor`,
  `divider`, `repeat`, `text`, `heading`, `badge`, `diff`).
- Chart kinds: `line`, `area`, `bar`, `stacked_bar`, `pie`, `donut`,
  `gauge`, `heatmap` (reserved), `custom`.
- Data sources: `series`, `series_by_kind`, `rows`,
  `series_from_rsql`, `static`.
- Schema export already wired:
  [`crates/starter-ui-ir/src/schema.rs`](../../crates/starter-ui-ir/src/schema.rs)
  exposes `schemars::schema_for!(ComponentTree)`.

### Builder DSL

- `crates/starter-ui-builder/` is a typed fluent builder (`dashboard()`,
  `kpi_grid()`, `line_chart()`, `row()`, `col()`, …). Compile-time
  guarantees source ↔ kind pairing.
- Zero-I/O by contract — only constructs IR.
- **Not exposed to the agent** in any form. The agent has to hand-roll
  raw JSON.

### Bindings

- `crates/starter-ui-bindings/` resolves `{{...}}` expressions over an
  `EntityGraph` (`$target`, `$stack`, `$self`, `$user`, `$page`).
- Documented in
  [`DOCS/frontend/sdui/SCOPE.md`](../../../DOCS/frontend/sdui/SCOPE.md);
  divergence from rubix noted in
  [`DOCS/frontend/sdui/DIVERGENCE.md`](../../../DOCS/frontend/sdui/DIVERGENCE.md).

### Dashboard tools

| Tool                       | File                                                                                                | Validates `body_json`?        |
| -------------------------- | --------------------------------------------------------------------------------------------------- | ----------------------------- |
| `rubix.dashboard.create`   | [`create.rs`](../../crates/rubix-tools/src/dashboard/create.rs)                                     | **YES** (just landed)         |
| `rubix.dashboard.update`   | [`update.rs`](../../crates/rubix-tools/src/dashboard/update.rs)                                     | **NOT YET** — TODO this PR    |
| `rubix.dashboard.page_set` | [`page_set.rs`](../../crates/rubix-tools/src/dashboard/page_set.rs)                                 | n/a (slot write, not body)    |
| `rubix.dashboard.get`      | [`get.rs`](../../crates/rubix-tools/src/dashboard/get.rs)                                           | n/a (read)                    |
| `rubix.dashboard.list`     | [`list.rs`](../../crates/rubix-tools/src/dashboard/list.rs)                                         | n/a (read)                    |
| `rubix.dashboard.delete`   | [`delete.rs`](../../crates/rubix-tools/src/dashboard/delete.rs)                                     | n/a                           |
| `rubix.dashboard.duplicate`| [`duplicate.rs`](../../crates/rubix-tools/src/dashboard/duplicate.rs)                               | n/a (copies an existing body) |
| `rubix.dashboard.history`  | [`history.rs`](../../crates/rubix-tools/src/dashboard/history.rs)                                   | n/a                           |

### Skill

- [`rubix/crates/rubix-skills/skills/dashboard-builder/SKILL.md`](../../crates/rubix-skills/skills/dashboard-builder/SKILL.md)
  hard-codes three intent → layout templates (IoT, Disk, System
  Health). Anything off that path is a guessing game.

## What is missing — the tool gap

Treat each bullet as a new tool to add to `rubix-tools` (or
`starter-tool-*` for the generic ones). Names are working titles.

### Discovery (so the agent stops guessing the schema)

- **`sdui.palette.list`** — return the full list of component `type`
  tags with one-line descriptions, grouped (`layout`, `display`,
  `interaction`, …). Derive from `starter_ui_ir` so it cannot drift.
- **`sdui.component.schema`** — `(type) -> JSON Schema` for one
  component variant. Lets the agent fetch the exact shape of `kpi` or
  `chart` on demand instead of inlining 35 schemas into the skill.
- **`sdui.chart.kinds`** — list `ChartKind` × valid `ChartSource`
  pairings (the compile-time matrix in `starter-ui-builder`).
- **`sdui.bindings.grammar`** — short description of `{{...}}`,
  qualifiers, and length-prefixed path traversal — pulled from
  `starter-ui-bindings` doc strings.

### Validation (so failure is loud and useful)

- **`sdui.tree.validate`** — `(body_json) -> { ok, diagnostics[] }`.
  Runs the same `serde_json::from_value::<ComponentTree>` we now do
  inside `create`, plus structural checks:
  - duplicate component `id`s
  - unknown chart kinds
  - kind ↔ source mismatch
  - dangling bindings (`{{$target.foo}}` with no `$target` in scope)
  - chart series referencing missing nodes
- **`sdui.tree.lint`** — best-practice warnings (KPI without unit,
  chart with no title, page with no children, more than 4 KPIs in a
  row, …). Non-fatal; surfaced to the chat so it can self-correct.

### Preview (so the agent can see what it built)

- **`sdui.tree.render_html`** — server-side render the tree to a
  self-contained HTML snippet via `starter-export` (already a dep on
  the dashboard side). Returned as a blob ref so the chat UI can
  embed an `<iframe srcdoc>`. The agent gets visual confirmation
  without needing a live frontend round-trip.
- **`sdui.tree.snapshot_png`** — optional, headless-render the same
  HTML to a PNG via the existing export pipeline. Useful for "show me
  what you made" in narration.

### Data dry-run (so charts don't render empty)

- **`sdui.chart.dry_run`** — `(ChartSource) -> { ok, sample_rows,
  row_count, latency_ms }`. Executes the underlying ClickHouse / RSQL
  query with `LIMIT 10` and returns the first rows. The agent can
  confirm that `iot.messages.rate` actually has data before wiring
  the chart.
- **`warehouse.series.list`** — already half-built in
  `starter-warehouse`; surface it as a tool so the agent can enumerate
  available metric series instead of inventing `iot.devices.online`.
- **`clickhouse.tables.list`** + **`clickhouse.table.schema`** — same
  idea for raw tables behind `rows` / `series_from_rsql` sources.

### Cloning / diffing (so good dashboards seed new ones)

- **`sdui.dashboard.search`** — search dashboards by component type,
  chart kind, tag, owner. Today `list` only does pagination.
- **`sdui.dashboard.diff`** — `(page_id_a, page_id_b) -> jsonpatch`.
  The agent can say "show me how the working `disk-overview`
  structures its KPI row" and then mimic.
- **`sdui.dashboard.fork`** — server-side: clone existing dashboard,
  apply a JSON patch, validate, persist. One round trip instead of
  three.

### Builder bridge (so the agent uses the typed DSL, not raw JSON)

- **`sdui.builder.compile`** — accept a small structured DSL request
  (e.g. `{ "kind": "kpi_grid", "kpis": [...] }`) and run it through
  `starter-ui-builder` server-side, returning the generated
  `ComponentTree`. Lets the LLM stay at the recipe level and never
  emit raw IR.
- Long-term: codegen the builder's typed API as MCP tool definitions
  directly. The compile-time kind ↔ source guarantees become tool
  schema constraints.

## Why this matters for "powerful BI dashboards"

The current loop is:

```
chat → write raw jsonb → ❌ resolver 404 → user sees broken page
```

What we want:

```
chat → palette.list / component.schema  (discover)
     → builder.compile or hand-write IR
     → tree.validate + tree.lint        (cheap, no DB write)
     → chart.dry_run on each source     (cheap, real data)
     → tree.render_html                 (preview in chat)
     → dashboard.create                 (persist, already validated)
     → narrate with the rendered HTML blob ref
```

Every step is a separate tool the LLM can compose. Each one is
individually cheap and reversible. The agent never persists an
unrenderable body again, and "build me an IoT dashboard" stops
being a 5-stack-trace debugging session.

## Order of work

1. **Stop the bleeding** *(this PR, in flight)*
   - `dashboard.update` body validation (mirror what `create` just got).
   - Add `sdui.tree.validate` as a thin tool wrapper — same code path.
2. **Discovery surface** — `palette.list`, `component.schema`,
   `chart.kinds`. All read-only, all derived from `starter-ui-ir`. No
   new tests beyond "returns non-empty".
3. **Data dry-run** — `chart.dry_run` + `warehouse.series.list`. The
   highest-leverage one: it kills the "chart renders empty" failure
   mode.
4. **Preview** — `tree.render_html` first (cheap), `snapshot_png`
   only if we actually need it for narration.
5. **Builder bridge** — `sdui.builder.compile`. Once this exists the
   skill collapses from "33 inlined templates" to "call
   `builder.compile`".
6. **Search / diff / fork** — quality-of-life; only worth doing once
   (1)–(5) are in.

## Open questions

- Do we expose these as MCP tools (i.e. behind `mcp__acme__*`) or as
  REST endpoints first? MCP is consistent with the dashboard CRUD
  surface and means the chat-agent picks them up automatically.
- `tree.render_html` needs the SDUI renderer running server-side.
  Today the renderer is React-only. Options: (a) bundle a headless
  React runner via `deno_core` / `rquickjs`, (b) build a minimal Rust
  renderer that covers the subset the agent generates (probably
  enough for KPI grids + charts), (c) shell out to a tiny
  `node`-based renderer that the agent process spawns. (c) is the
  fastest path; (b) is the cleanest.
- `chart.dry_run` needs auth/tenant scoping — the agent must not be
  able to dry-run a query it could not legitimately render.

## Files referenced

- [`crates/starter-ui-ir/src/lib.rs`](../../../crates/starter-ui-ir/src/lib.rs)
- [`crates/starter-ui-ir/src/chart.rs`](../../../crates/starter-ui-ir/src/chart.rs)
- [`crates/starter-ui-ir/src/schema.rs`](../../../crates/starter-ui-ir/src/schema.rs)
- [`crates/starter-ui-builder/src/lib.rs`](../../../crates/starter-ui-builder/src/lib.rs)
- [`crates/starter-ui-bindings/src/lib.rs`](../../../crates/starter-ui-bindings/src/lib.rs)
- [`DOCS/frontend/sdui/SCOPE.md`](../../../DOCS/frontend/sdui/SCOPE.md)
- [`DOCS/frontend/sdui/DIVERGENCE.md`](../../../DOCS/frontend/sdui/DIVERGENCE.md)
- [`rubix/crates/rubix-tools/src/dashboard/`](../../crates/rubix-tools/src/dashboard/)
- [`rubix/crates/rubix-skills/skills/dashboard-builder/SKILL.md`](../../crates/rubix-skills/skills/dashboard-builder/SKILL.md)
- Prior context:
  [`2026-05-25-dashboards-sidebar-sse-and-chat-gaps.md`](2026-05-25-dashboards-sidebar-sse-and-chat-gaps.md),
  [`2026-05-25-dashboard-assistant-e2e.md`](2026-05-25-dashboard-assistant-e2e.md).
