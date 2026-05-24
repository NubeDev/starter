# @nube/starter-ui-dashboard

Dashboard primitives — animated metric tiles, radial progress, performance
charts, and live activity feeds. Pure presentation; no I/O, no router, no
i18n runtime. Pair with `@nube/starter-ui-kit` for the primitive layer.

## Install

```bash
pnpm add @nube/starter-ui-dashboard
```

Peers: `@nube/starter-ui-kit`, `motion`, `lucide-react`, `react`, `tailwindcss@4`.

## Tokens

All components target the standard shadcn token surface — `--card`,
`--card-foreground`, `--muted`, `--muted-foreground`, `--primary`,
`--destructive`, `--border`. If you've imported
`@nube/starter-ui-kit/styles.css` you're done.

Accent colours (sparkline strokes, ring fills, activity-feed icon
tints) accept any CSS colour string — pass a hex, `hsl(var(--your-token))`,
or a named colour. Each component falls back to `hsl(var(--primary))`
or `currentColor` if you omit it.

## Quick start

```tsx
import { MetricCard, RadialProgress, ActivityFeed, PerformanceChart } from "@nube/starter-ui-dashboard";
import { Leaf, Droplet } from "lucide-react";

<MetricCard
  label="Revenue"
  value={42300}
  prefix="$"
  delta={12.4}
  spark={[2, 4, 3, 6, 8, 7, 9]}
  accent="hsl(var(--primary))"
/>

<RadialProgress value={68} label="Battery" subLabel="12h remaining" />

<PerformanceChart
  title="Energy harvested"
  headline="42.3"
  headlineSuffix="kWh"
  delta="↑ 12.4%"
  data={[2, 4, 3, 6, 8, 7, 9]}
  labels={["M","T","W","T","F","S","S"]}
  periods={["1D","1W","1M","1Y"]}
  activePeriodIndex={1}
  onPeriodChange={(i) => console.log(i)}
/>

<ActivityFeed
  title="Activity"
  streamingLabel="Streaming"
  nowLabel="now"
  items={[
    { id: "a", icon: Leaf, title: "Air upgraded", meta: "Cabin +12%", time: "0m", accent: "#4ade80" },
    { id: "b", icon: Droplet, title: "Water filter", meta: "98% clean", time: "2m", accent: "#67e8f9" },
  ]}
/>
```

## i18n

Labels are props, never `useIntl()`. Translate at the call site:

```tsx
const intl = useIntl();
<MetricCard label={intl.formatMessage({ id: "metric.revenue" })} value={...} />
```

This keeps the package free of `react-intl` (or any i18n framework)
and lets each consumer decide how localization works.

## R6 compliance

Zero I/O, zero side effects beyond `setInterval` (auto-rotation in
`ActivityFeed`, opt-out via `intervalMs={0}`). No global state, no
context consumption. Drop into any React tree.
