// `toggle.tsx` — writable boolean point rendered as a (read-only here) switch.
import * as React from "react";
import { Switch } from "../ui/switch";
import type { WidgetProps } from "./registry";

export function ToggleWidget({ title, value }: WidgetProps): React.ReactElement {
  const on = value === undefined ? false : Boolean(value);
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-sm text-foreground">{title}</span>
      <Switch checked={on} label={title} disabled />
    </div>
  );
}
