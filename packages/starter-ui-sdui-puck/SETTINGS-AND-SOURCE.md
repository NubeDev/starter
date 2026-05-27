# Component Settings & Source Configuration

How component settings are generated, rendered, and resolved at runtime.

## Field Generation Pipeline

`build-puck-config.ts` walks the IR JSON Schema (`definitions.Component.oneOf`).
For each variant it calls `armToComponentConfig()` → iterates properties →
`propertyToField()` per prop. The dispatch order:

| Priority | Condition | Puck Field Type |
|----------|-----------|-----------------|
| 1 | `(variant, propertyPath)` in `curation/data-sources.ts` | `custom` → `<DataSourceField>` catalogue picker |
| 2 | `(variant, propertyPath)` in `curation/slots.ts` | `slot` (drag target) |
| 3 | Schema `enum` (string) | `select` with options |
| 4 | `string` | `text` |
| 5 | `number` / `integer` | `number` |
| 6 | `boolean` | `radio` (Yes/No) |
| 7 | `array` | `array` with nested `arrayFields` |
| 8 | `$ref` (non-Component) | `text` (fallback; object selector deferred to PR2) |
| 9 | Inline `object` with `properties` | `object` with recursive `objectFields` |
| 10 | `oneOf` at property level | `text` (union picker deferred to PR2) |
| 11 | Unknown | `text` fallback |

Most component settings (strings, numbers, enums, booleans, nested objects,
arrays) are handled automatically by priorities 3–9. No curation needed.

## The `source` Field (ChartSource)

`source` on `kpi` (and `sources[]` on `chart`) is a **discriminated union**
(`ChartSource`, tagged by `type`):

```
Static            → inline (ts_ms, value) points
AnalyticsTemplate → { name, params?, map? }
Series            → live telemetry path
SeriesByKind      → kind-based telemetry
Rows              → tabular query
SeriesFromRsql    → RSQL filter expression
```

Because `oneOf` at property level falls through to a plain text field
(priority 10), the `source` property is **special-cased** in
`curation/data-sources.ts`:

```ts
{ variant: "kpi", propertyPath: "source", kind: "analytics_template" }
```

This forces it through the `<DataSourceField>` catalogue picker (priority 1).

### What the Picker Does

1. On mount, calls `catalogue.list("analytics_template")` from context.
2. If the host returns entries → renders a `<select>` dropdown.
3. If the host throws → degrades to a free-text `<input>` + warning banner.
4. **Wire format**: reads/writes `{ type: "analytics_template", name: "<selected>", ...rest }`.
   The `writeWireValue()` function preserves sibling keys (`map`, `params`)
   when the user picks a new template name.

### Host Catalogue (in `$pageId_.edit.tsx`)

The edit route supplies catalogue handlers via `catalogueFromMap({...})`:

```ts
analytics_template: async () => {
  // TODO: replace once rubix.analytics.template.list ships
  throw new Error('…')  // → degrades to free-text
},
tool: async () => { /* fetches /api/v1/tools */ },
tenant: async () => { /* client.tenantList() */ },
unit_symbol: async () => [ /* hardcoded list */ ],
page_state_key: async () => [ /* $page.range_from, $page.range_to */ ],
```

The `analytics_template` handler currently **always throws**, so operators
see the free-text degradation for source picking.

## Current DATA_SOURCES Entries

| variant | propertyPath | kind | Effect |
|---------|-------------|------|--------|
| `kpi` | `source` | `analytics_template` | Template-name picker |
| `kpi` | `unit_symbol` | `unit_symbol` | Unit dropdown (kWh, °C, …) |
| `sparkline` | `unit_symbol` | `unit_symbol` | Same |
| `action_widget` | `action_ref` | `tool` | Tool-name picker from `/api/v1/tools` |
| `drawer` | `open` | `page_state_key` | Page-state key dropdown |

## Runtime Resolution (Server-Side)

When `<SduiPage>` calls `POST /ui/resolve`:

1. `resolve_chart_sources()` in `crates/starter-sdui-routes/src/chart_resolve.rs`
   walks the tree.
2. For `ChartSource::AnalyticsTemplate` → invokes `AnalyticsBridge` (calls
   `rubix.analytics.query`), maps rows via `source.map` → scalar or series.
3. For `ChartSource::Static` → extracts inline data points.
4. Result: `node.value` (KPI) or `node.series` (chart) populated on the
   resolved tree returned to the client.

## Gaps / Known Issues

| Issue | Status |
|-------|--------|
| Only the `analytics_template` arm of `ChartSource` is editable via picker | PR2 union picker needed for other arms |
| `chart.sources` (array of ChartSource) has no DATA_SOURCES entry | Falls through to auto-generated array-of-text |
| `$ref` to non-Component objects → plain text | PR2 |
| `analytics_template` catalogue handler always throws | Wire to real verb or hardcode known templates |
| Sub-fields of source (`map`, `params`) not editable in picker UI | Must edit JSON directly or wait for PR2 |

## Adding a New Catalogue-Backed Field

1. Add an entry to `DATA_SOURCES` in `src/curation/data-sources.ts`:
   ```ts
   { variant: "my_widget", propertyPath: "my_prop", kind: "tool" }
   ```
2. Ensure the host's `catalogueFromMap()` has a handler for that `kind`.
3. Done — the generator picks it up automatically.
