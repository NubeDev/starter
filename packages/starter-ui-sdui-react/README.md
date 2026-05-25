# @nube/starter-ui-sdui-react

Headless React renderer for starter SDUI trees. Mount one
`<SduiPage>`, supply an `SduiTransport` via `<SduiProvider>`, and the
package dispatches one renderer per IR variant.

## Mount

```tsx
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StarterClient } from "@nube/starter-client-ts";
import {
  SduiPage,
  SduiProvider,
  createHttpSduiTransport,
} from "@nube/starter-ui-sdui-react";

const qc = new QueryClient();
const client = new StarterClient({ baseUrl: "/" });
const transport = createHttpSduiTransport({ client });

export function DashboardRoute() {
  return (
    <QueryClientProvider client={qc}>
      <SduiProvider transport={transport}>
        <SduiPage pageRef="dashboard.overview" />
      </SduiProvider>
    </QueryClientProvider>
  );
}
```

`<SduiPage>` owns the page-state bag, runs `useSduiResolve` keyed by
it, opens the subscription plan, and dispatches the root node into
the renderer registry.

## Transport seam

The package never imports `fetch`, never reads `process.env`, never
hard-codes a URL. Every network call rides on the injected
`SduiTransport`:

```ts
interface SduiTransport {
  resolve(req): Promise<UiResolveResponse>;
  action(req): Promise<UiActionResponse>;
  table(req): Promise<TableResponse>;
  subscribe(subjects, onUpdate): () => void;
}
```

`createHttpSduiTransport({ client })` wraps a `StarterClient` and
calls `POST <apiPrefix>/ui/{resolve|action|table}`. The default
`subscribe()` polls every 15 s; hosts that ship server-sent events
override `transport.subscribe()` directly.

Custom renderers (e.g. `rubix.alarm-table`) plug in via
`<SduiProvider customRenderers={{ "rubix.alarm-table": MyImpl }}>`.

## Renderer per variant

One file per IR variant lives in `src/renderer/render-*.tsx`. Each
file registers itself with the central dispatcher on import; the
public barrel imports them for side effects:

| Variant       | File                       |
|---------------|----------------------------|
| `page`        | `render-page.tsx`          |
| `grid`/`kpi_grid` | `render-grid.tsx`      |
| `kpi`         | `render-kpi.tsx`           |
| `chart`/`sparkline` | `render-chart.tsx`   |
| `table`       | `render-table.tsx`         |
| `form`        | `render-form.tsx`          |
| `tabs`        | `render-tabs.tsx`          |
| `select`      | `render-select.tsx`        |
| `slider`      | `render-slider.tsx`        |
| `toggle`      | `render-toggle.tsx`        |
| `date_range`  | `render-date-range.tsx`    |
| `divider`     | `render-divider.tsx`       |
| `custom`      | `render-custom.tsx`        |
| `repeat`      | `render-repeat.tsx`        |

Unknown variants fall back to a `data-sdui-dangling` placeholder so
adding a new IR variant server-side doesn't crash older clients.

## Scripts

- `pnpm -F @nube/starter-ui-sdui-react typecheck` — strict TS.
- `pnpm -F @nube/starter-ui-sdui-react test` — Vitest, one smoke per
  renderer using `react-dom/server::renderToStaticMarkup`.
