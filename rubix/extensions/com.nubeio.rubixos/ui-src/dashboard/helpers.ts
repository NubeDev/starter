// Pure formatting / inference helpers.  No React, no DOM.

import type { MeterRow } from "../types";

export function fmtBig(v: number): string {
  if (!Number.isFinite(v)) return "—";
  const abs = Math.abs(v);
  // Always 3-4 sig figs + SI suffix. Anything past trillions keeps
  // the T suffix rather than wrapping into scientific — operators
  // care about magnitude not exact value at that scale.
  const fmt = (n: number) => (Math.abs(n) >= 100 ? n.toFixed(0)
                            :  Math.abs(n) >= 10  ? n.toFixed(1)
                            :                       n.toFixed(2));
  if (abs >= 1e12) return fmt(v / 1e12) + "T";
  if (abs >= 1e9)  return fmt(v / 1e9)  + "B";
  if (abs >= 1e6)  return fmt(v / 1e6)  + "M";
  if (abs >= 1e3)  return fmt(v / 1e3)  + "k";
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
