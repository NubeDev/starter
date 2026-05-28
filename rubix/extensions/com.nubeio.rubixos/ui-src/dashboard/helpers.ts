// Pure formatting / inference helpers.  No React, no DOM.

import type { MeterRow } from "../types";

export function fmtBig(v: number): string {
  if (!Number.isFinite(v)) return "—";
  const abs = Math.abs(v);
  if (abs >= 1e9) return (v / 1e9).toFixed(2) + "B";
  if (abs >= 1e6) return (v / 1e6).toFixed(2) + "M";
  if (abs >= 1e3) return (v / 1e3).toFixed(2) + "k";
  return v.toFixed(abs >= 10 ? 1 : 2);
}

export function inferUnit(meters: ReadonlyArray<MeterRow>): string | null {
  const counts = new Map<string, number>();
  for (const m of meters) {
    if (m.unit) counts.set(m.unit, (counts.get(m.unit) ?? 0) + 1);
  }
  let best: string | null = null;
  let bestN = 0;
  for (const [u, n] of counts) {
    if (n > bestN) { best = u; bestN = n; }
  }
  return best;
}

// Parse Australian state suffix from a `sites-geo` locality
// string like "Yatala, QLD".  Returns "—" when missing.
export function stateOf(locality: string | null | undefined): string {
  if (!locality) return "—";
  const i = locality.lastIndexOf(",");
  return (i >= 0 ? locality.slice(i + 1) : locality).trim() || "—";
}
