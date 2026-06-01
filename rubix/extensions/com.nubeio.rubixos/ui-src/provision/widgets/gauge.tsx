// `gauge.tsx` — radial-ish gauge using a stroked SVG arc. Mock value.
import * as React from "react";
import type { WidgetProps } from "./registry";

export function GaugeWidget({ title, value, unit }: WidgetProps): React.ReactElement {
  const pct = typeof value === "number" ? Math.max(0, Math.min(100, value)) : 62;
  const radius = 34;
  const circ = Math.PI * radius; // half circle
  const dash = (pct / 100) * circ;
  return (
    <figure className="flex flex-col items-center gap-1">
      <svg viewBox="0 0 90 52" className="w-full max-w-[120px]">
        <path
          d="M 11 48 A 34 34 0 0 1 79 48"
          fill="none"
          stroke="currentColor"
          className="text-muted"
          strokeWidth="8"
          strokeLinecap="round"
        />
        <path
          d="M 11 48 A 34 34 0 0 1 79 48"
          fill="none"
          stroke="currentColor"
          className="text-primary"
          strokeWidth="8"
          strokeLinecap="round"
          strokeDasharray={`${dash} ${circ}`}
        />
      </svg>
      <div className="text-lg font-semibold tabular-nums text-foreground">
        {typeof value === "number" ? value : "—"}
        {unit ? <span className="text-xs text-muted-foreground"> {unit}</span> : null}
      </div>
      <figcaption className="text-xs text-muted-foreground">{title}</figcaption>
    </figure>
  );
}
