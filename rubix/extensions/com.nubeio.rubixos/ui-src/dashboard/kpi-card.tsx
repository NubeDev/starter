import * as React from "react";

export function KpiCard({
  eyebrow, value, sub, unit, accent, className, deltaPct, deltaLabel,
}: {
  eyebrow: string;
  value: string;
  sub?: string;
  unit?: string | null;
  accent?: boolean;
  className?: string;
  /** Period-over-period change, in percent (e.g. 12.3 for +12.3%).
   *  `null` hides the badge — used when prior data is missing. */
  deltaPct?: number | null;
  deltaLabel?: string;
}): React.ReactElement {
  const showDelta = deltaPct !== undefined && deltaPct !== null && Number.isFinite(deltaPct);
  // Up = bad (more consumption), down = good. Tone neutrally if |Δ| < 0.5%.
  const tone =
    !showDelta ? ""
    : Math.abs(deltaPct!) < 0.5 ? "text-muted-foreground border-border/40 bg-muted/20"
    : deltaPct! > 0           ? "text-amber-300 border-amber-400/40 bg-amber-400/10"
                              : "text-emerald-300 border-emerald-400/40 bg-emerald-400/10";
  const arrow = !showDelta ? "" : deltaPct! > 0 ? "↑" : deltaPct! < 0 ? "↓" : "→";
  return (
    <div className={"ext-glass p-3 " + (accent ? "ext-glass--accent " : "") + (className ?? "")}>
      <div className="ext-eyebrow">{eyebrow}</div>
      <div className="ext-num text-2xl font-semibold leading-tight mt-1">
        {value}
        {unit ? <span className="text-sm text-muted-foreground ml-1">{unit}</span> : null}
      </div>
      {showDelta ? (
        <div className="mt-1.5 flex items-center gap-1.5">
          <span
            className={
              "inline-flex items-center gap-1 px-1.5 py-0.5 text-[0.65rem] tabular-nums " +
              "rounded-md border " + tone
            }
          >
            <span aria-hidden="true">{arrow}</span>
            {Math.abs(deltaPct!).toFixed(1)}%
          </span>
          {deltaLabel ? (
            <span className="text-[0.65rem] text-muted-foreground">{deltaLabel}</span>
          ) : null}
        </div>
      ) : null}
      {sub ? <div className="text-[0.7rem] text-muted-foreground mt-0.5">{sub}</div> : null}
    </div>
  );
}
