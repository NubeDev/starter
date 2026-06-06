// `grid` / `kpi_grid` — responsive CSS grid; `columns` controls layout.
import { cn } from "@nube/starter-ui-kit";
import { RenderChildren } from "../headless/render.js";
import { registerRenderer } from "../headless/registry.js";
import { nodeStyleAttrs } from "./node-style.js";

export function RenderGrid({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const cols = typeof node.columns === "number" ? node.columns : 3;
  return (
    <div
      {...nodeStyleAttrs(node.style)}
      className={cn("sdui-grid grid gap-3", node.style?.className)}
      style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}
    >
      <RenderChildren nodes={node.children} />
    </div>
  );
}

registerRenderer("grid", RenderGrid);
registerRenderer("kpi_grid", RenderGrid);
