// `row` — 12-column responsive grid container. Children are typically
// `col` nodes whose `span` (1–12) maps to `col-span-N` Tailwind
// classes. Mirrors the Bootstrap-style row/col idiom the bundled
// dashboard JSON uses (`crates/rubix-flows/dashboards/*.json`).
import { cn } from "@nube/starter-ui-kit";
import { RenderChildren } from "../headless/render.js";
import { registerRenderer } from "../headless/registry.js";
import { nodeStyleAttrs } from "./node-style.js";

export function RenderRow({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  return (
    <div
      {...nodeStyleAttrs(node.style)}
      className={cn(
        "sdui-row grid grid-cols-12 gap-4 sm:gap-5",
        node.style?.className,
      )}
    >
      <RenderChildren nodes={node.children} />
    </div>
  );
}

registerRenderer("row", RenderRow);
