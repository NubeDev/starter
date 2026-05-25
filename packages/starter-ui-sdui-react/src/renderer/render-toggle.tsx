// `toggle` — boolean switch bound to `page_state`.
import { Switch, Label, cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "./registry.js";
import { usePageStateKey } from "../page-state.js";

export function RenderToggle({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const key = typeof node.page_state_key === "string" ? node.page_state_key : null;
  const label = typeof node.label === "string" ? node.label : undefined;
  const [value, setValue] = usePageStateKey(key ?? "_unkeyed_toggle");
  const current = value === true;
  const id = node.id ?? `toggle-${key ?? "unkeyed"}`;
  return (
    <div className={cn("sdui-toggle flex items-center gap-2", node.style?.className)}>
      <Switch id={id} checked={current} onCheckedChange={(v) => setValue(v)} />
      {label ? <Label htmlFor={id}>{label}</Label> : null}
    </div>
  );
}

registerRenderer("toggle", RenderToggle);
