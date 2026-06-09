// Time-range + auto-refresh state barrel (WS-01). Re-exports only.

export { useTimeStore, DEFAULT_RANGE } from "@/store/time/store";
export type { RefreshSecs } from "@/store/time/store";
export {
  resolveTimeRange,
  resolveBound,
} from "@/store/time/resolve";
export type { TimeRange, TimeBound, ResolvedRange } from "@/store/time/resolve";
export { intervalSecs, DEFAULT_TARGET_POINTS } from "@/store/time/interval";
