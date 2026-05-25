// `date_range` — two native date inputs writing `{ from, to }` into
// `page_state[node.page_state_key]`. The fancier calendar popover
// is a v2 enhancement.
import { Input, Label, cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "./registry.js";
import { usePageStateKey } from "../page-state.js";

interface Range { from?: string; to?: string }

export function RenderDateRange({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const key = typeof node.page_state_key === "string" ? node.page_state_key : null;
  const label = typeof node.label === "string" ? node.label : undefined;
  const [value, setValue] = usePageStateKey(key ?? "_unkeyed_range");
  const current: Range = (value && typeof value === "object" ? (value as Range) : {});
  const write = (patch: Range) => setValue({ ...current, ...patch });
  return (
    <div className={cn("sdui-date-range flex flex-col gap-1", node.style?.className)}>
      {label ? <Label>{label}</Label> : null}
      <div className="flex items-center gap-2">
        <Input
          type="date"
          value={current.from ?? ""}
          onChange={(e) => write({ from: e.target.value })}
        />
        <span className="text-muted-foreground">→</span>
        <Input
          type="date"
          value={current.to ?? ""}
          onChange={(e) => write({ to: e.target.value })}
        />
      </div>
    </div>
  );
}

registerRenderer("date_range", RenderDateRange);
