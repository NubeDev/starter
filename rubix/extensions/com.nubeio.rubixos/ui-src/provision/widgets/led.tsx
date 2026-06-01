// `led.tsx` — status indicator dot (on/off/fault). Mock state from value.
import * as React from "react";
import type { WidgetProps } from "./registry";

export function LedWidget({ title, value }: WidgetProps): React.ReactElement {
  const on = value === undefined ? true : Boolean(value);
  return (
    <div className="flex items-center gap-2">
      <span
        className={
          "inline-block size-3 rounded-full " +
          (on ? "bg-emerald-500 shadow-[0_0_8px] shadow-emerald-500/60" : "bg-muted-foreground/40")
        }
        aria-hidden
      />
      <span className="text-sm text-foreground">{title}</span>
      <span className="ml-auto text-xs text-muted-foreground">{on ? "ON" : "OFF"}</span>
    </div>
  );
}
