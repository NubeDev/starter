# SDUI visual design contract

The single source of truth for how SDUI renderers (React web,
React Native, Flutter) should look. Implemented first in the React
renderer at
[`packages/starter-ui-sdui-react/src/renderer/`](../../../../packages/starter-ui-sdui-react/src/renderer/);
RN and Flutter follow.

## 1. Tokens

Renderers must NOT hardcode colors. All visual surfaces resolve to a
rubix theme token so palette switching (nube / ocean / sunset) and
mode switching (light / dark) work without renderer changes.

### Accent roles (semantic, not literal)

| Role  | Web CSS var          | Flutter (extension) | Use                                  |
|-------|----------------------|---------------------|--------------------------------------|
| leaf  | `--color-leaf`       | `accentLeaf`        | Primary/positive series, first KPI   |
| aqua  | `--color-aqua`       | `accentAqua`        | Cool series, second KPI              |
| sun   | `--color-sun`        | `accentSun`         | Energy/highlight, third KPI          |
| sky   | `--color-sky`        | `accentSky`         | Info, fourth KPI                     |
| warn  | `--color-warn`       | `accentWarn`        | Cautionary — opt-in via `intent`     |
| ok    | `--color-ok`         | `statusOk`          | Positive delta/trend                 |
| danger| `--color-danger`     | `statusDanger`      | Negative delta/trend                 |

### Surface

| Role          | Web                     | Flutter                       |
|---------------|-------------------------|-------------------------------|
| Card frame    | `.glass` class          | `RubixGlassDecoration()`      |
| Card radius   | `rounded-3xl` (1.5rem)  | `BorderRadius.circular(24)`   |
| Card padding  | `p-5 sm:p-6`            | `EdgeInsets.all(20–24)`       |
| Hairline top  | gradient via `--color-leaf` (or accent) at `inset-x-5 top-0 h-px` | 1px container at top, accent color |
| Glow blob     | `-right-12 -top-12 h-32 w-32 rounded-full opacity-40 blur-2xl` color-mix with accent at 55% | `BoxDecoration` with `RadialGradient` clipped to card |

### Typography

| Role            | Web                                                          | Flutter                                       |
|-----------------|--------------------------------------------------------------|-----------------------------------------------|
| Page title      | `text-3xl sm:text-4xl font-medium tracking-[-0.02em]`        | `headlineMedium` weight 500, letter-spacing -.5 |
| Eyebrow         | `text-[11px] font-semibold uppercase tracking-[0.22em]` in `--color-leaf` | `labelSmall` upper, spacing 2.2, accentLeaf   |
| KPI label       | `text-[11px] font-semibold uppercase tracking-[0.18em]` in `--color-subtle` | `labelSmall` upper, spacing 1.8, onSurfaceVariant |
| KPI value       | `text-4xl sm:text-5xl font-medium tracking-[-0.03em] .tabular`, color = accent | `displaySmall` weight 500, `FontFeature.tabularFigures()`, color=accent |
| KPI unit        | `text-sm font-medium` in `--color-muted`                      | `titleSmall`, muted                            |
| Chart title     | `text-sm font-semibold tracking-[-0.01em]`                    | `titleSmall` weight 600                       |

## 2. Accent resolution

Renderers pick an accent for each KPI / chart series in this order:

1. **Explicit `node.accent`** — if `leaf | aqua | sun | sky | warn`, use it.
2. **`node.intent`** mapped via:
   ```
   primary | positive | good → leaf
   info                       → sky
   warn | warning             → warn
   energy                     → sun
   cool                       → aqua
   ```
3. **Hash fallback** — `hash(node.id) % [leaf, aqua, sun, sky]`
   (skip `warn` — reserved for explicit intent). Gives siblings
   stable, visually-distinct colors across re-renders.
4. **Within `kpi_grid`** — items without explicit accent/intent get
   `accentByIndex(i)` instead of the hash, so the first row reads
   leaf / aqua / sun / sky in order.

Reference implementation:
[`packages/starter-ui-sdui-react/src/renderer/accent.ts`](../../../../packages/starter-ui-sdui-react/src/renderer/accent.ts).

## 3. KPI anatomy

```
┌─ glass card, rounded-3xl ─────────────────┐
│ ┄┄┄┄┄┄ hairline (accent gradient) ┄┄┄┄┄┄  │← accent
│                            ╲ glow blob    │  (top 1px)
│ LABEL · UPPERCASE · 11px                  │← subtle
│                                           │
│ 10,014.523   kWh                          │← accent (value)
│                                           │  muted (unit)
│ +2.4% ↑                                   │← ok / danger
└───────────────────────────────────────────┘
```

Status delta colors:
- `trend` starts with `+` or matches `/^up\b/i` → `--color-ok`
- `trend` starts with `-` or matches `/^down\b/i` → `--color-danger`
- Otherwise muted.

## 4. Chart palette

Renderers must derive series colors from theme tokens, not a frozen
literal palette. Series cycle through:

```
[leaf, aqua, sun, sky, warn]
```

Axes / grid / ticks:
- stroke = `--color-muted`
- grid = `--color-border`
- font = `--font-sans`, 11px
- area fill under line = `color-mix(in oklab, <stroke> 14%, transparent)`
- line width = 2, no point markers (line clarity > dot density)

Reference: [`render-chart.tsx`](../../../../packages/starter-ui-sdui-react/src/renderer/render-chart.tsx)
reads tokens via `getComputedStyle(host)` at mount.

Empty state: dashed rounded card,
`border-color: color-mix(in oklab, var(--color-muted) 40%, transparent)`,
copy `"no data"` in `--color-muted`.

## 5. Layout density

| Token        | Web                          | Flutter                  |
|--------------|------------------------------|--------------------------|
| Page padding | `p-4 sm:p-6` + `gap-6`       | `EdgeInsets.all(16/24)` + `SizedBox(height: 24)` between sections |
| Row gap      | `gap-4 sm:gap-5`             | `mainAxisSpacing: 16/20` |
| Col gap      | `gap-4` (between stacked widgets) | `crossAxisSpacing: 16` |

## 6. Per-platform port checklist

### React (done)

- [x] `render-page.tsx` — eyebrow + larger title
- [x] `render-kpi.tsx` — glass + accent + hairline + glow + status trend
- [x] `render-kpi-grid.tsx` — same anatomy, `accentByIndex` rotation
- [x] `render-chart.tsx` — theme-resolved palette, styled axes, area fill
- [x] `render-row` / `render-col` — slightly larger gap tokens
- [x] `accent.ts` — shared accent resolver

### Flutter (done)

- [x] `SduiTheme extends ThemeExtension<SduiTheme>` with `accentLeaf /
  accentAqua / accentSun / accentSky / accentWarn / statusOk /
  statusDanger / glassFill / glassBorder / hairline / subtleText /
  mutedText`. Ships `.light` and `.dark` defaults so the package
  works without host wiring; host apps override via
  `ThemeData(extensions: [...])`.
  ([sdui_theme.dart](../../../flutter/packages/rubix_sdui/lib/src/widgets/sdui_theme.dart))
- [x] `accent.dart` — Dart port of `accent.ts` with the identical
  hash function (`(h * 31 + c).toSigned(32)`) and intent map.
  ([accent.dart](../../../flutter/packages/rubix_sdui/lib/src/widgets/components/accent.dart))
- [x] Glass card frame via `_SduiGlassCard`: `ClipRRect` + `Stack`
  with a `RadialGradient` glow blob top-right, a gradient hairline on
  the top edge, and the surface fill from `glassFill`. Card radius 24.
- [x] `SduiKpiWidget` rewritten: glass frame, accent-tinted value with
  `FontFeature.tabularFigures()`, label letter-spacing 1.8, trend
  auto-coloured by `+`/`-` prefix.
- [x] `SduiKpiGridWidget` added (was a TODO in the dispatcher) and
  wired into `buildComponent`. Uses `accentByIndex(i)` for the
  default rotation.
- [x] `SduiChartWidget` rewritten: cycles series strokes through the
  five accent colors, fills 14%-opacity area under each line, draws
  themed grid lines from `glassBorder`, axis labels from `mutedText`.
- [x] `SduiPageWidget` supports optional `eyebrow` field with the
  accent-leaf hairline + tracked caps line.
- [x] Row/col gap bumped 12→16, page padding 16→20/24 for parity.

### React Native (done)

- [x] `accent.ts` exported from `@nube/starter-ui-sdui-react/headless`
  so the RN package shares the resolver verbatim.
- [x] [accent-colors.ts](../../../../packages/starter-ui-sdui-native/src/accent-colors.ts) —
  per-mode (`light`/`dark`) concrete hex values for each `SduiAccent`,
  mirroring `SduiTheme.light`/`.dark` defaults. Local to the package
  until `@nube/starter-ui-kit-native` grows rubix-specific accent
  tokens.
- [x] [render-kpi.tsx](../../../../packages/starter-ui-sdui-native/src/render-kpi.tsx)
  wraps the kit `Card` in a `Box` that paints a 2px accent strip on
  the top edge (kit `Card` doesn't accept `style`); the value text is
  tinted in the accent color with `fontVariant: ['tabular-nums']`;
  trend auto-coloured via `trendColor`.
- [x] [render-kpi-grid.tsx](../../../../packages/starter-ui-sdui-native/src/render-kpi-grid.tsx)
  added and registered in the package barrel (was deferred-with-web
  in the original index comment).
- [x] [render-page.tsx](../../../../packages/starter-ui-sdui-native/src/render-page.tsx)
  picks up the optional `eyebrow` field with the accent-leaf coloured
  caps tracking.
- [x] Bonus: fixed a pre-existing test failure by adding
  `accessibilityRole="main"` to the page Column.

## 7. What renderers MUST NOT do

- Hardcode colors. Always go through theme tokens / `accent.ts`.
- Use `Card` from shadcn directly for SDUI surfaces — the SDUI glass
  card shape is custom (rounded-3xl + glow + hairline). `Card` is fine
  for the rest of the app.
- Force a chart palette that ignores the active theme.
- Drop `tabular-nums` from KPI values (digits jitter when data
  refreshes via SSE).
- Use scale transforms on hover/press — they cause layout shift in
  grids. Use color/opacity transitions only.

## 8. Verification

Headless screenshot script: see
[`scripts/sdui-screens.mjs`](../../../../scripts/sdui-screens.mjs)
(TODO — checked-in version of the helper used during the polish pass).
Pages to capture for visual regression:
- `/dashboards/data-flow-site-a` in light + dark
- `/dashboards/disk-overview` (KPI grid heavy)
