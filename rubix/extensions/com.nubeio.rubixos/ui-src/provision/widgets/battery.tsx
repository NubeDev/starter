// `battery.tsx` — horizontal battery fill bar. Mock value (% of full).
import * as React from "react";
import { BatteryMedium } from "lucide-react";
import type { WidgetProps } from "./registry";

export function BatteryWidget({ title, value }: WidgetProps): React.ReactElement {
  const pct = typeof value === "number" ? Math.max(0, Math.min(100, value)) : 78;
  const tone = pct < 20 ? "bg-destructive" : pct < 50 ? "bg-yellow-500" : "bg-primary";
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <BatteryMedium className="size-4" />
        <span>{title}</span>
      </div>
      <div className="h-3 w-full overflow-hidden rounded-full bg-muted">
        <div className={`h-full ${tone}`} style={{ width: `${pct}%` }} />
      </div>
      <span className="text-sm font-medium tabular-nums text-foreground">{pct}%</span>
    </div>
  );
}
