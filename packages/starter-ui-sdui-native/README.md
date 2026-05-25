# @nube/starter-ui-sdui-native

React Native SDUI renderers. Importing this package for its side
effects registers one renderer per IR `Kind` into the shared
`registerRenderer` registry exported by
`@nube/starter-ui-sdui-react/headless`. Mobile apps then mount the
existing `<SduiPage>` from that headless subpath and get a native
render.

```ts
import "@nube/starter-ui-sdui-native";              // ← side-effect registration
import { SduiPage, SduiProvider } from "@nube/starter-ui-sdui-react/headless";
```

## Architecture

Each `render-<kind>.tsx`:

- imports **only** `@nube/starter-ui-kit-native` and
  `@nube/starter-ui-ir` type-level imports — **never** `react-native`
  directly. This keeps the kit as the single styling seam and makes
  every renderer testable against a mock kit.
- is ≤ 150 lines.
- calls `registerRenderer(kind, RenderX)` at module-load time, against
  the registry in `@nube/starter-ui-sdui-react/headless`.
- mirrors the priority order in
  [`packages/starter-ui-sdui-react/src/renderer/index.ts`](../starter-ui-sdui-react/src/renderer/index.ts)
  one-for-one.

## Kinds covered

The 16 kinds the web renderer registers today are all covered here:

| # | kind         | file                  |
|---|--------------|-----------------------|
|  1 | `page`        | `render-page.tsx`        |
|  2 | `row`         | `render-row.tsx`         |
|  3 | `col`         | `render-col.tsx`         |
|  4 | `grid`        | `render-grid.tsx`        |
|  5 | `kpi`         | `render-kpi.tsx`         |
|  6 | `chart`       | `render-chart.tsx`       |
|  7 | `divider`     | `render-divider.tsx`     |
|  8 | `tabs`        | `render-tabs.tsx`        |
|  9 | `table`       | `render-table.tsx`       |
| 10 | `form`        | `render-form.tsx`        |
| 11 | `select`      | `render-select.tsx`      |
| 12 | `slider`      | `render-slider.tsx`      |
| 13 | `toggle`      | `render-toggle.tsx`      |
| 14 | `date_range`  | `render-date-range.tsx`  |
| 15 | `repeat`      | `render-repeat.tsx`      |
| 16 | `custom`      | `render-custom.tsx`      |

## Kinds **deferred** (parity backfill, web-first)

The IR declares 26 kinds in
[`packages/starter-ui-ir/src/index.ts`](../starter-ui-ir/src/index.ts).
The 10 the **web** renderer does not register today are deferred
here too, deliberately, per
[`rubix/docs/scope/mobile/NEW-PACKAGES.md §Parity vs the IR Kind union`](../../rubix/docs/scope/mobile/NEW-PACKAGES.md):

- `stack`
- `card`
- `text`
- `heading`
- `badge`
- `kpi_grid`
- `button`
- `link`
- `field`
- `sparkline`

A phone surface should not silently render IR shapes the
source-of-truth web renderer rejects. If mobile ever needs one of
the 10 before web does, that's a separate decision and gets called
out in the PR — it is NOT silently registered.

> Note: the web `render-chart.tsx` registers `sparkline` as an alias
> of `chart`, and `render-grid.tsx` registers `kpi_grid` as an alias
> of `grid`. Per the parity rule above, **this package does not
> mirror those alias registrations** — both are listed as deferred
> in the spec, so we wait for an explicit web-first decision.
