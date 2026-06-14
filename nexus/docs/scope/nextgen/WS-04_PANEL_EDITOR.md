# WS-04 — Panel Editor & Visualization Config

> **Status:** Not started · **Wave:** 1 (independent) · **Owner:** _unassigned_
> **Depends on:** C1 JSON model (for `fieldConfig`/overrides) · benefits from WS-03 editor + WS-01 preview
> **Migration:** none (panel config is in the dashboard/panel model) · **Read first:** GAP_ANALYSIS §2.4, ROADMAP §0
> **Verified:** `nexus-gaps` tip on 2026-06-09 — evidence re-grepped; all "Current state" claims
> hold. Note: panel config rides in the backend's opaque `layout` JSON (`panelAdapter.ts`), so the
> `fieldConfig`/`transforms` model extension is UI-only — no DTO/migration (confirms "Migration: none").
>
> 🔗 **Coordinate with [WS-11 (Units & Prefs)](./WS-11_UNITS_AND_PREFS.md).** The "unit picker" in
> the Field tab becomes **"pick a `quantity`"** (temperature/power/…) — the *displayed* unit then
> comes from the viewer's resolved prefs (server-converted), with an optional per-panel display
> override. Thresholds/min/max become **unit-aware** (store canonical, show in the viewer's unit).
> Add `quantity` + `storedUnit` to `SeriesField` *with* WS-11 so the model lands once.

## Goal
A Grafana-class **panel editor**: full control over visualization type, field mapping, units,
thresholds, axes, legend, per-series overrides, value mappings, and **field transforms**, with a
**live preview** that re-renders as you edit. Most of the rendering support **already exists** —
the renderers read these fields today; this WS is largely the missing **edit UI + state**.

## Current state (evidence)
- **High value-for-effort:** the data model already has `thresholds`, `min`, `max`, `decimals`,
  per-`SeriesField` `unit`/`label`/`color`/`kind`, and `xKind` — `ui/src/data/types.ts:42-91`.
- The **renderers already consume them**: `widgets/gaugeOption.ts` reads thresholds/min/max,
  `widgets/lineOption.ts` formats time axes & legend.
- But the editor (`canvas/PanelProperties.tsx`) only exposes: viz type switch, SQL, x-column, and
  the **first series' value column**. Thresholds/units/decimals/min/max/per-series/legend/axes have
  **no edit UI**. 10 viz types are catalogued (`widgets/catalog.ts`) but under-configured.

## Scope
1. **Full-screen panel editor** (`ui/src/features/canvas/PanelEditor/**`, new) — replace/augment the
   side `PanelProperties` with a Grafana-style layout: query area (top, uses WS-03 editor) + **live
   preview** + a right-hand **options inspector** with tabs:
   - **Query** — datasource, SQL (WS-03 CodeMirror), run/preview, stats.
   - **Visualization** — pick type (all 10), with per-type option sets.
   - **Field** — unit picker (grouped: SI, data rate, temp, %, currency, time…), decimals, min/max,
     thresholds editor (add/remove steps + colors), value mappings (value/range → text/color),
     no-value display.
   - **Overrides** — per-series / per-column overrides (match by name or regex → set color, unit,
     axis, display name, hidden). Stored as `fieldConfig.overrides` (extend C1 model).
   - **Legend & Axes** — legend on/off/placement/values, Y-axis scale (linear/log), soft min/max,
     axis labels, multiple Y axes.
2. **Multi-series editing** — today only series[0].value is editable; let users add/remove/rename
   series, map each to a column, set color/unit/axis.
3. **Field transforms** (`ui/src/features/canvas/transforms/**`, new) — a client-side pipeline applied
   to query rows before render: rename fields, add calculated field, filter, group-by/aggregate,
   join/merge results, organize/reorder, reduce (to single value for stat/gauge). Composable list UI.
4. **Live preview** — debounced re-render on any config change using the current query result;
   refetch only when the query itself changes.
5. **Per-type option polish** — sensible options for table (column widths, cell coloring by
   threshold, pagination), stat (sparkline, color mode, text size), gauge (orientation), bar
   (stacking/orientation), pie (donut/labels), heatmap (color scale).

## Design notes
- **Extend, don't fork, `WidgetConfig`** — add `fieldConfig: { defaults, overrides[] }` and
  `transforms: Transform[]` to the model (C1). Keep the existing flat `thresholds/min/max/decimals`
  working or migrate them into `fieldConfig.defaults` with a back-compat read.
- **Reuse the option-builders** in `widgets/*Option.ts` — they already map config→ECharts; this WS
  feeds them richer config, it does not rewrite the chart layer.
- **Transforms are pure functions** over `WidgetData.points` — unit-test each in isolation
  (mirrors the existing F10 pure-logic test discipline).
- Unit registry: a shared list of unit ids + formatters (some formatting already exists via
  `useDateTime`/preferences — extend, don't duplicate).

## Acceptance criteria
- [ ] User can set unit, decimals, min/max, and a multi-step threshold from the UI; gauge/stat/table
  reflect them immediately in preview.
- [ ] User can add a second series, map it to a column, set its color/axis.
- [ ] An override (e.g. `series matching /temp/ → unit °C, red`) applies.
- [ ] At least 4 transforms work (rename, calculated field, group-by, reduce) and are testable.
- [ ] Live preview re-renders on config change without refetching the query.
- [ ] Back-compat: existing dashboards render unchanged after the model extension.
- [ ] Tests: transform functions, fieldConfig→option mapping, override matching.

## Out of scope (hand off)
- The SQL editor internals → WS-03 (consume it).
- Time range / variables in the query → WS-01 / WS-02.
- New viz *types* beyond the existing 10 (e.g. geomap, node graph) → future WS (note the gap).
