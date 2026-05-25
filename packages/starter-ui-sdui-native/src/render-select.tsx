// `select` — controlled by `page_state[node.page_state_key]`.
import type { UiComponent } from "@nube/starter-ui-ir";
import {
  Column,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  Text,
} from "@nube/starter-ui-kit-native";
import { registerRenderer, usePageStateKey } from "@nube/starter-ui-sdui-react/headless";

interface Opt { value: string; label?: string }

export function RenderSelect({ node }: { node: UiComponent }) {
  const key = typeof node.page_state_key === "string" ? node.page_state_key : null;
  const label = typeof node.label === "string" ? node.label : undefined;
  const opts: Opt[] = Array.isArray(node.options)
    ? (node.options as Opt[]).filter((o) => typeof o?.value === "string")
    : [];
  const [value, setValue] = usePageStateKey(key ?? "_unkeyed_select");
  const current = typeof value === "string" ? value : undefined;
  const placeholder = typeof node.placeholder === "string" ? node.placeholder : undefined;
  return (
    <Column gap={4} testID={(node.id as string | undefined) ?? "sdui-select"}>
      {label ? <Text variant="label">{label}</Text> : null}
      <Select
        value={current}
        onValueChange={(v: string) => setValue(v)}
      >
        <SelectTrigger
          placeholder={placeholder}
          accessibilityLabel={label}
        />
        <SelectContent>
          {opts.map((o) => (
            <SelectItem key={o.value} value={o.value}>
              {o.label ?? o.value}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Column>
  );
}

registerRenderer("select", RenderSelect);
