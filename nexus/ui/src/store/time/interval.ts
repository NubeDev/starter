// Derive a sensible `$__interval` bucket width from a resolved window, so
// `$__timeGroup(col, $__interval)` produces roughly `targetPoints` buckets
// (Grafana's auto-interval). The binder takes `interval_secs`; this is the
// client side of that contract.
//
// The width is snapped to a "nice" step (1/2/5/10/30 s, m, h, …) so axis
// labels land on readable boundaries instead of arbitrary durations.

import type { ResolvedRange } from "@/store/time/resolve";

/** Default bucket target — panels are typically a few hundred px wide, and
 *  ~200 points keeps charts dense without oversampling the source. */
export const DEFAULT_TARGET_POINTS = 200;

// Ascending "nice" bucket widths in seconds.
const NICE_SECONDS = [
  1, 2, 5, 10, 15, 30,
  60, 120, 300, 600, 900, 1800,
  3600, 7200, 10800, 21600, 43200,
  86400, 172800, 604800,
];

/** Bucket width in whole seconds for the given window and point target.
 *  Always at least 1s. Snaps up to the nearest nice step so a 6h window
 *  buckets to e.g. 2m rather than 108s. */
export function intervalSecs(
  range: ResolvedRange,
  targetPoints: number = DEFAULT_TARGET_POINTS,
): number {
  const spanSecs = Math.max(1, (range.to.getTime() - range.from.getTime()) / 1000);
  const raw = spanSecs / Math.max(1, targetPoints);
  for (const step of NICE_SECONDS) {
    if (step >= raw) return step;
  }
  return NICE_SECONDS[NICE_SECONDS.length - 1];
}
