# Feature: Chart / Panel Settings — Reference & Test Dashboard

> Verified: nexus-rewrite, 2026-06-10. Companion to
> [DASHBOARDS.md](DASHBOARDS.md). Covers the panel editor's 7 tabs, what each
> setting does, the config key it writes, and how it persists.

The panel editor (open a panel in a dashboard's **edit mode** → its properties /
the full-screen editor) has these tabs:
**Query · Visualization · Field · Overrides · Legend & Axes · Transforms**.

## The "one widget per setting" test dashboard

A dashboard exists where **each panel isolates one setting and is named after
it**, so you can eyeball them all at once:

```
http://localhost:4790/d/chart-settings?from=now-6h&to=now
```

(Set the time picker to a window that has data — datapump publishes "now"-ish, so
last 1–6h. The panels query `telemetry_typed` for `site-001`.) 28 panels, one per
setting listed below. **Report any whose name doesn't match what it shows.**

---

## ⚠️ Persistence model (read this — it caused a real bug)

The backend `nexus_panels` table has only `sql`, `viz`, `layout` (jsonb). It has
**no column** for the display config. So the **entire** chart config — field
mapping, `fieldConfig` (unit/decimals/thresholds/overrides), `options`
(legend/axes), and `transforms` — is **stashed inside the opaque `layout` blob**
by `ui/src/api/dashboards/panelAdapter.ts`.

**Bug found & fixed 2026-06-10:** the adapter previously stashed **only
`fields`**, so every Field / Overrides / Legend / Transforms edit was silently
**dropped on save** (it showed in the live preview, then vanished on reload). Fix:
`stashLayout`/`readLayout` now round-trip the full display config. Guarded by
`panelAdapter.test.ts` ("round-trips the full display config", "preserves
fieldConfig/options/transforms through a save+reload").

> When building panels **via the API** (not the editor UI), you must hand-build
> the `layout` blob: `{ x,y,w,h, fields:{x,series}, fieldConfig?, options?,
> transforms? }`. The editor does this for you.

---

## Tab 1 — Query
| Setting | Does | Persists to |
|---------|------|-------------|
| Title | Panel heading | `panel.title` (own column) |
| Datasource | Which datasource the SQL runs against | `panel.datasource_id` (own column) |
| SQL | The query; supports `$__timeFilter/$__timeGroup/$__interval/$var` | `panel.sql` (own column) |
| Test run | Runs the query, shows row count/cols/timing — no mutation | — |

## Tab 2 — Visualization
| Setting | Does | Persists to | Test widget |
|---------|------|-------------|-------------|
| Visualization type | line/area/bar/stat/gauge/pie/table | `panel.viz` | `viz: line` … `viz: pie` |
| X column | Result column for the category/time axis (omit for stat/gauge) | `layout.fields.x` | every line/bar panel |
| Series (add/remove/label/column) | Which result columns are drawn | `layout.fields.series[]` | `viz: multi-series` |

Behavior verified by `features/widgets/lineOption.test.ts` (x axis + series
mapping, area mode), `renderWidget.test.tsx` (every type mounts).

## Tab 3 — Field (defaults for every series)
Writes `layout.fieldConfig.defaults`.
| Setting | Does | Key | Test widget | Unit test |
|---------|------|-----|-------------|-----------|
| Unit | Append/scale a unit (kWh, °C, %, bytes…) | `defaults.unit` | `field: unit kWh` | `formatValue.test.ts` |
| Decimals | Fixed fraction digits (blank = auto) | `defaults.decimals` | `field: decimals 0/3` | `formatValue.test.ts` |
| Min / Max | Axis/gauge bounds (blank = auto) | `defaults.min/max` | `field: min/max 0-100` | `gaugeOption.test.ts` |
| No-value display | Text shown for null/NaN | `defaults.noValue` | `field: no-value text` | `formatValue.test.ts` |
| Thresholds ramp | Colour steps by value | `defaults.thresholds[]` | `field: thresholds ramp` | `thresholdState.test.ts`, `rampColor.test.ts` |
| Value mappings | Map a value/range/regex → text+colour | `defaults.mappings[]` | `field: value mapping` | `formatValue.test.ts` |

## Tab 4 — Overrides (per-series exceptions)
Writes `layout.fieldConfig.overrides[]` — first match wins.
| Setting | Does | Key | Test widget | Unit test |
|---------|------|-----|-------------|-----------|
| Matcher (by name / by regex) | Which series the override targets | `overrides[].matcher` | all override panels | `fieldConfig.test.ts` |
| Display name | Rename a series | `overrides[].display.displayName` | `override: rename series` | `fieldConfig.test.ts` |
| Colour | Recolour a series | `overrides[].display.color` | `override: colour series` | `lineOption.test.ts` |
| Hidden | Drop a series from the chart | `overrides[].display.hidden` | `override: hide series` | `lineOption.test.ts` |
| Unit (per series) | Override unit for one series | `overrides[].display.unit` | (covered by field unit) | `fieldConfig.test.ts` |

## Tab 5 — Legend & Axes
Writes `layout.options`. Single-value panels (stat/gauge) ignore these.
| Setting | Does | Key | Test widget | Unit test |
|---------|------|-----|-------------|-----------|
| Legend on/off | Show the series legend | `options.legend.show` | `legend: hidden` | `cartesianChrome.test.ts` |
| Placement | top / right / bottom | `options.legend.placement` | `legend: placement right/bottom` | `cartesianChrome.test.ts` |
| Y-axis scale | linear / log | `options.yAxis.scale` | `axis: log scale` | `cartesianChrome.test.ts` |
| Soft min / max | Soft axis bounds | `options.yAxis.softMin/softMax` | `axis: soft min/max` | `cartesianChrome.test.ts` |
| Y-axis label | Axis title | `options.yAxis.label` | `axis: label` | `cartesianChrome.test.ts` |

## Tab 6 — Transforms (client-side, applied to rows before render)
Writes `layout.transforms[]`, run by `features/canvas/transforms/`.
| Setting | Does | Test widget | Unit test |
|---------|------|-------------|-----------|
| filter | Keep rows matching field op value | `transform: filter` | `transforms.test.ts` |
| reduce | Collapse a column to one value (avg/sum/last…) | `transform: reduce avg` | `transforms.test.ts` |
| rename | Rename a column | `transform: rename` | `transforms.test.ts` |
| calculated | New column = left op right | `transform: calculated` | `transforms.test.ts` |
| groupBy | Group + aggregate | (add a panel to test) | `transforms.test.ts` |
| organize | Reorder/drop columns | (add a panel to test) | `transforms.test.ts` |

---

## Status of verification

**Verified in a real browser (Playwright, 2026-06-10): all 28 panels render
correctly.** The spec `ui/e2e/chart-settings.spec.ts` loads `/d/chart-settings`,
classifies every panel (rendered / no-data / error), and asserts none fail. Run:

```bash
cd nexus/ui && pnpm exec playwright test --project=chromium
# (needs the stack up + the chart-settings dashboard seeded + datapump running)
```

Exact formatted values were probed and confirmed: `decimals 3 → 20.253`,
`unit → 20.25 kWh`, `value mapping 1 → On ⚡`, `no-value → n/a`.

### Bugs this testing found and fixed
1. **Panel persistence dropped the whole display config.** `panelAdapter` stashed
   only `fields`, so Field/Overrides/Legend/Transforms edits vanished on save.
   Fixed to round-trip the full config (see the persistence section above);
   `panelAdapter.test.ts`.
2. **Stat/gauge ignored `noValue`** — showed generic "No data" instead of the
   configured text. Fixed in `Stat.tsx`.
3. **Stat/gauge bypassed `formatValue`** — decimals, unit symbols, and value
   mappings did **not** apply to stat tiles (they worked on tables). Fixed by
   routing the stat value through `formatValue` and adding `display` / `valueColor`
   props to the shared `MetricCard` (`packages/starter-ui-dashboard/src/metric-card.tsx`).
   Locked in by `Stat.test.tsx` (decimals / unit / mapping / no-value).

- Per-setting *behaviour* is also covered by the deterministic unit suite under
  `ui/src/features/widgets/` + `features/canvas/transforms/` (cited above).

## Gotchas for authoring panels via the API (not the editor)
- **Unit ids are registry ids, not symbols.** Use `kilowatthour` (→ "kWh"), not
  `kwatth`. The full list is `ui/src/features/widgets/units.ts`. An unknown id
  silently renders no symbol.
- **ValueMapping is flat**: `{type, match, text, color}` — there is no `result`
  wrapper. A value mapping only shows on a **numeric** stat (a non-numeric base
  value makes the stat empty before mapping).

## Known gaps / to add
- `groupBy` and `organize` transforms have no dedicated test panel yet — add two.
- Per-series `unit` override shares the field-unit panel; add a dedicated one if
  you want it isolated.
