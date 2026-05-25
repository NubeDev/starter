// `slider` — single-thumb numeric slider bound to `page_state`.
import { Slider, Label, cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";
import { usePageStateKey } from "../headless/page-state.js";

export function RenderSlider({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const key = typeof node.page_state_key === "string" ? node.page_state_key : null;
  const label = typeof node.label === "string" ? node.label : undefined;
  const min = typeof node.min === "number" ? node.min : 0;
  const max = typeof node.max === "number" ? node.max : 100;
  const step = typeof node.step === "number" ? node.step : 1;
  const [value, setValue] = usePageStateKey(key ?? "_unkeyed_slider");
  const current = typeof value === "number" ? value : min;
  return (
    <div className={cn("sdui-slider flex flex-col gap-1", node.style?.className)}>
      {label ? (
        <Label>
          {label}: <span className="tabular-nums">{current}</span>
        </Label>
      ) : null}
      <Slider
        value={[current]}
        min={min}
        max={max}
        step={step}
        onValueChange={(vals) => setValue(vals[0])}
      />
    </div>
  );
}

registerRenderer("slider", RenderSlider);
