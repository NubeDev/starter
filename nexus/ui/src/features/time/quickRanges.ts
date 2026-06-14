// The quick-range catalogue offered by the picker. Each entry is a label
// plus a relative {from,to} expressed in the same token grammar the resolver
// understands, so picking one stores the relative tokens (they keep tracking
// `now` across refreshes) rather than freezing an absolute window.

import type { TimeRange } from "@/store/time/resolve";

export interface QuickRange {
  label: string;
  range: TimeRange;
}

/** Ordered quick ranges, Grafana-style. Relative tokens only. */
export const QUICK_RANGES: ReadonlyArray<QuickRange> = [
  { label: "Last 5 minutes", range: { from: "now-5m", to: "now" } },
  { label: "Last 15 minutes", range: { from: "now-15m", to: "now" } },
  { label: "Last 1 hour", range: { from: "now-1h", to: "now" } },
  { label: "Last 6 hours", range: { from: "now-6h", to: "now" } },
  { label: "Last 24 hours", range: { from: "now-24h", to: "now" } },
  { label: "Last 7 days", range: { from: "now-7d", to: "now" } },
  { label: "Last 30 days", range: { from: "now-30d", to: "now" } },
  { label: "Today", range: { from: "now/d", to: "now" } },
  // Yesterday = [start of yesterday, start of today). The `to` is `now/d`, NOT
  // `now-1d/d` — both being `now-1d/d` collapsed to a single instant ("From must
  // be before to" → empty range, no data). Same start-vs-end pattern for the
  // last-week / last-month ranges below.
  { label: "Yesterday", range: { from: "now-1d/d", to: "now/d" } },
  { label: "This week", range: { from: "now/w", to: "now" } },
  { label: "Last week", range: { from: "now-1w/w", to: "now/w" } },
  { label: "This month", range: { from: "now/M", to: "now" } },
  { label: "Last month", range: { from: "now-1M/M", to: "now/M" } },
];

/** The refresh-interval options, in seconds (`0` = off). */
export const REFRESH_OPTIONS: ReadonlyArray<{ label: string; secs: number }> = [
  { label: "Off", secs: 0 },
  { label: "5s", secs: 5 },
  { label: "10s", secs: 10 },
  { label: "30s", secs: 30 },
  { label: "1m", secs: 60 },
  { label: "5m", secs: 300 },
  { label: "15m", secs: 900 },
];

/** Human label for the current range: the matching quick-range label, else a
 *  compact echo of the raw tokens (absolute ranges show as `from -> to`). */
export function rangeLabel(range: TimeRange): string {
  const hit = QUICK_RANGES.find(
    (q) => q.range.from === range.from && q.range.to === range.to,
  );
  if (hit) return hit.label;
  return `${range.from} -> ${range.to}`;
}
