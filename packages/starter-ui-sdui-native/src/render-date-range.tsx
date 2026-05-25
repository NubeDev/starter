// `date_range` — two text inputs writing `{ from, to }` into
// `page_state[node.page_state_key]`. RN has no `<input type=date>`;
// a native date-picker overlay is a v2 enhancement.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Column, Input, Row, Text } from "@nube/starter-ui-kit-native";
import { registerRenderer, usePageStateKey } from "@nube/starter-ui-sdui-react/headless";

interface Range { from?: string; to?: string }

export function RenderDateRange({ node }: { node: UiComponent }) {
  const key = typeof node.page_state_key === "string" ? node.page_state_key : null;
  const label = typeof node.label === "string" ? node.label : undefined;
  const [value, setValue] = usePageStateKey(key ?? "_unkeyed_range");
  const current: Range = value && typeof value === "object" ? (value as Range) : {};
  const write = (patch: Range) => setValue({ ...current, ...patch });
  return (
    <Column gap={4} testID={(node.id as string | undefined) ?? "sdui-date-range"}>
      {label ? <Text variant="label">{label}</Text> : null}
      <Row gap={8}>
        <Input
          value={current.from ?? ""}
          onChangeText={(v: string) => write({ from: v })}
          placeholder="YYYY-MM-DD"
          accessibilityLabel={label ? `${label} from` : "from"}
        />
        <Text variant="body" color="muted">
          →
        </Text>
        <Input
          value={current.to ?? ""}
          onChangeText={(v: string) => write({ to: v })}
          placeholder="YYYY-MM-DD"
          accessibilityLabel={label ? `${label} to` : "to"}
        />
      </Row>
    </Column>
  );
}

registerRenderer("date_range", RenderDateRange);
