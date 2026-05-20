import type { SlotKind } from "../types.js";

/**
 * Stable colour mapping per slot kind. Used by both handles and edges
 * so a typed connection visibly carries its type from source to
 * target. Hosts can override via CSS by targeting
 * `[data-slot-kind="<kind>"]`.
 */
export const SLOT_COLORS: Record<SlotKind, string> = {
  any: "#94a3b8", // slate-400
  string: "#3b82f6", // blue-500
  number: "#10b981", // emerald-500
  boolean: "#a855f7", // purple-500
  json: "#f59e0b", // amber-500
  bytes: "#64748b", // slate-500
  event: "#ec4899", // pink-500
  trigger: "#ef4444", // red-500
  stream: "#06b6d4", // cyan-500
};

export function colorForKind(kind: string): string {
  return SLOT_COLORS[kind as SlotKind] ?? SLOT_COLORS.any;
}
