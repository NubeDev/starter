// `stat.tsx` — big number with a label. Mock value.
import * as React from "react";
import type { WidgetProps } from "./registry";

export function StatWidget({ title, value, unit }: WidgetProps): React.ReactElement {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs uppercase tracking-wide text-muted-foreground">{title}</span>
      <span className="text-2xl font-semibold tabular-nums text-foreground">
        {value ?? "—"}
        {unit ? <span className="ml-1 text-sm text-muted-foreground">{unit}</span> : null}
      </span>
    </div>
  );
}
