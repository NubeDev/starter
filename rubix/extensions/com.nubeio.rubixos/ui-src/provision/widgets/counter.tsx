// `counter.tsx` — monotonic counter readout (totalizer). Mock value.
import * as React from "react";
import { Gauge } from "lucide-react";
import type { WidgetProps } from "./registry";

export function CounterWidget({ title, value, unit }: WidgetProps): React.ReactElement {
  const shown = value ?? 128450;
  return (
    <div className="flex items-center gap-3">
      <Gauge className="size-5 text-muted-foreground" />
      <div className="flex flex-col">
        <span className="text-xs text-muted-foreground">{title}</span>
        <span className="font-mono text-lg tabular-nums text-foreground">
          {shown}
          {unit ? <span className="ml-1 text-xs text-muted-foreground">{unit}</span> : null}
        </span>
      </div>
    </div>
  );
}
