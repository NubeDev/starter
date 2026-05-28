import * as React from "react";

import { Sparkline } from "../sparkline";
import { fmtBig } from "./helpers";

export function SiteTile({
  name, locality, total, last, pct, spark, unit, selected, onClick,
}: {
  name: string;
  locality: string | null;
  total: number;
  last: number | null;
  pct: number;
  spark: ReadonlyArray<number | null>;
  unit: string | null;
  selected: boolean;
  onClick: () => void;
}): React.ReactElement {
  return (
    <button
      type="button"
      onClick={onClick}
      className={
        "ext-glass text-left p-4 cursor-pointer transition-transform duration-150 " +
        "hover:-translate-y-0.5 hover:shadow-xl focus:outline-none focus-visible:ring-2 focus-visible:ring-primary " +
        (selected ? "ext-glass--accent" : "opacity-90 hover:opacity-100")
      }
      aria-pressed={selected}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="text-sm font-semibold truncate">{name}</div>
          <div className="ext-eyebrow truncate">{locality ?? "—"}</div>
        </div>
        <div className="text-right">
          <div className="ext-num text-lg font-semibold leading-none">
            {fmtBig(last ?? 0)}
            {unit ? <span className="text-xs text-muted-foreground ml-1">{unit}</span> : null}
          </div>
          <div className="ext-eyebrow mt-1">latest</div>
        </div>
      </div>

      <div className="mt-3 -mx-1 h-9 text-primary">
        <Sparkline values={spark} width={300} height={36} color="currentColor" />
      </div>

      <div className="mt-2 flex items-center gap-2">
        <div className="h-1.5 flex-1 rounded-full bg-muted/40 overflow-hidden">
          <div
            className="h-full rounded-full bg-primary/70"
            style={{ width: `${Math.max(2, pct)}%` }}
          />
        </div>
        <div className="ext-num text-[0.7rem] text-muted-foreground tabular-nums w-20 text-right">
          Σ {fmtBig(total)}{unit ? ` ${unit}` : ""}
        </div>
      </div>
    </button>
  );
}
