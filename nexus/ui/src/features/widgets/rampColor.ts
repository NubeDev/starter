import type { ThresholdStep } from "@/data/types";

// Picks the colour for a value from a multi-step threshold ramp: the
// highest step whose `value` the reading meets or exceeds wins. The base
// step (`value: null`) acts as the floor. Steps need not be pre-sorted —
// we sort defensively, treating the base step as the lowest. Returns an
// hsl string ECharts/CSS can paint, or undefined when the ramp is empty or
// no step applies (so the caller keeps its default colour). Pure.
export function rampColor(
  value: number,
  steps: ReadonlyArray<ThresholdStep>,
): string | undefined {
  if (steps.length === 0) return undefined;
  const sorted = [...steps].sort(
    (a, b) => (a.value ?? -Infinity) - (b.value ?? -Infinity),
  );
  let chosen: ThresholdStep | undefined;
  for (const step of sorted) {
    const lower = step.value ?? -Infinity;
    if (value >= lower) chosen = step;
    else break;
  }
  return chosen ? `hsl(${chosen.color})` : undefined;
}
