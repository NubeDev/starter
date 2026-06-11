export type ThresholdState = "ok" | "warn" | "crit";

// Classify a value against warn/crit bounds. When `crit < warn` the
// metric is descending (lower is worse, e.g. battery charge); otherwise
// ascending (higher is worse, e.g. load). Both bounds are required for a
// verdict — a single bound leaves the state nominal.
export function thresholdState(
  value: number,
  warn?: number,
  crit?: number,
): ThresholdState {
  if (warn == null || crit == null) return "ok";
  const descending = crit < warn;
  if (descending) {
    if (value <= crit) return "crit";
    if (value <= warn) return "warn";
    return "ok";
  }
  if (value >= crit) return "crit";
  if (value >= warn) return "warn";
  return "ok";
}
