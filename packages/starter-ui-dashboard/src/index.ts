// # @nube/starter-ui-dashboard
//
// Reusable dashboard primitives — animated metric tiles, radial
// progress, performance charts, and live activity feeds. Pure
// presentation: no data fetching, no router coupling, no i18n
// runtime. All labels are passed in as props so the host owns the
// translation pipeline.
//
// Visual language follows shadcn/ui tokens (`--card`, `--primary`,
// `--muted-foreground`, …). Accent colours (sparkline strokes, ring
// fills, activity badges) accept arbitrary CSS colour strings so each
// consumer can wire them to whatever palette their theme exposes.
//
// Pair with `@nube/starter-ui-kit` for the primitive layer.

export { MetricCard } from "./metric-card.js";
export type { MetricCardProps } from "./metric-card.js";

export { RadialProgress } from "./radial-progress.js";
export type { RadialProgressProps } from "./radial-progress.js";

export { ActivityFeed } from "./activity-feed.js";
export type { ActivityFeedProps, ActivityItem } from "./activity-feed.js";

export { PerformanceChart } from "./performance-chart.js";
export type { PerformanceChartProps } from "./performance-chart.js";
