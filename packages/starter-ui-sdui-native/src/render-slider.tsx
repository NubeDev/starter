// `slider` — single-thumb numeric slider bound to `page_state`.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Column, Row, Slider, Text } from "@nube/starter-ui-kit-native";
import { registerRenderer, usePageStateKey } from "@nube/starter-ui-sdui-react/headless";

export function RenderSlider({ node }: { node: UiComponent }) {
  const key = typeof node.page_state_key === "string" ? node.page_state_key : null;
  const label = typeof node.label === "string" ? node.label : undefined;
  const min = typeof node.min === "number" ? node.min : 0;
  const max = typeof node.max === "number" ? node.max : 100;
  const step = typeof node.step === "number" ? node.step : 1;
  const [value, setValue] = usePageStateKey(key ?? "_unkeyed_slider");
  const current = typeof value === "number" ? value : min;
  return (
    <Column gap={4} testID={(node.id as string | undefined) ?? "sdui-slider"}>
      {label ? (
        <Row gap={4}>
          <Text variant="label">{label}</Text>
          <Text variant="label" weight="medium">
            {String(current)}
          </Text>
        </Row>
      ) : null}
      <Slider
        value={current}
        min={min}
        max={max}
        step={step}
        onValueChange={(v: number) => setValue(v)}
        accessibilityLabel={label}
      />
    </Column>
  );
}

registerRenderer("slider", RenderSlider);
