import { useMemo } from "react";

import type { InsightFunctionDoc } from "@/api/types";

// The curated function surface, grouped by category, as a clickable cheatsheet.
// Each entry shows its signature and summary; clicking inserts its runnable
// example into the editor at the cursor. This is the discoverability half of
// the Transform pane — the autocomplete is the in-flow half, both fed from the
// same catalogue (`useInsightFunctions`).
const CATEGORY_ORDER = [
  "select",
  "filter",
  "window",
  "shape",
  "resample",
  "anomaly",
] as const;

const CATEGORY_LABEL: Record<string, string> = {
  select: "Select",
  filter: "Filter",
  window: "Window",
  shape: "Shape",
  resample: "Resample",
  anomaly: "Anomaly",
};

export function FunctionCheatsheet({
  functions,
  onInsert,
}: {
  functions: InsightFunctionDoc[];
  onInsert: (example: string) => void;
}) {
  // Group by category, keeping the curated order; any unknown category falls to
  // the end so a newly-added bucket is still visible rather than dropped.
  const groups = useMemo(() => {
    const byCat = new Map<string, InsightFunctionDoc[]>();
    for (const fn of functions) {
      const list = byCat.get(fn.category) ?? [];
      list.push(fn);
      byCat.set(fn.category, list);
    }
    const known = CATEGORY_ORDER.filter((c) => byCat.has(c));
    const extra = [...byCat.keys()].filter(
      (c) => !CATEGORY_ORDER.includes(c as (typeof CATEGORY_ORDER)[number]),
    );
    return [...known, ...extra].map((cat) => ({
      cat,
      fns: byCat.get(cat) ?? [],
    }));
  }, [functions]);

  if (functions.length === 0) {
    return (
      <p className="px-1 text-xs text-muted-foreground">
        No functions available.
      </p>
    );
  }

  return (
    <div className="scrollbar-thin flex flex-col gap-3 overflow-auto">
      {groups.map(({ cat, fns }) => (
        <div key={cat} className="flex flex-col gap-1.5">
          <h4 className="text-[0.7rem] font-semibold uppercase tracking-wide text-muted-foreground">
            {CATEGORY_LABEL[cat] ?? cat}
          </h4>
          <ul className="flex flex-col gap-1">
            {fns.map((fn) => (
              <li key={fn.name}>
                <button
                  type="button"
                  onClick={() => onInsert(fn.example)}
                  title={`Insert: ${fn.example}`}
                  className="group w-full rounded-md border border-transparent px-2 py-1.5 text-left transition hover:border-border/60 hover:bg-accent/40"
                >
                  <code className="font-mono text-xs text-foreground">
                    {fn.signature}
                  </code>
                  <span className="mt-0.5 block text-xs text-muted-foreground">
                    {fn.summary}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}
