// `select` — controlled by `page_state[node.page_state_key]`.
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Label,
  cn,
} from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";
import { usePageStateKey } from "../headless/page-state.js";

interface Opt { value: string; label?: string }

export function RenderSelect({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const key = typeof node.page_state_key === "string" ? node.page_state_key : null;
  const label = typeof node.label === "string" ? node.label : undefined;
  const opts: Opt[] = Array.isArray(node.options)
    ? (node.options as Opt[]).filter((o) => typeof o?.value === "string")
    : [];
  const [value, setValue] = usePageStateKey(key ?? "_unkeyed_select");
  const current = typeof value === "string" ? value : undefined;
  return (
    <div className={cn("sdui-select flex flex-col gap-1", node.style?.className)}>
      {label ? <Label>{label}</Label> : null}
      <Select value={current} onValueChange={(v) => setValue(v)}>
        <SelectTrigger>
          <SelectValue placeholder={node.placeholder as string | undefined} />
        </SelectTrigger>
        <SelectContent>
          {opts.map((o) => (
            <SelectItem key={o.value} value={o.value}>
              {o.label ?? o.value}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

registerRenderer("select", RenderSelect);
