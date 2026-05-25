// `toggle` — boolean switch bound to `page_state`.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Row, Switch, Text } from "@nube/starter-ui-kit-native";
import { registerRenderer, usePageStateKey } from "@nube/starter-ui-sdui-react/headless";

export function RenderToggle({ node }: { node: UiComponent }) {
  const key = typeof node.page_state_key === "string" ? node.page_state_key : null;
  const label = typeof node.label === "string" ? node.label : undefined;
  const [value, setValue] = usePageStateKey(key ?? "_unkeyed_toggle");
  const current = value === true;
  return (
    <Row gap={8} testID={(node.id as string | undefined) ?? "sdui-toggle"}>
      <Switch
        checked={current}
        onCheckedChange={(v: boolean) => setValue(v)}
        accessibilityLabel={label}
      />
      {label ? <Text variant="body">{label}</Text> : null}
    </Row>
  );
}

registerRenderer("toggle", RenderToggle);
