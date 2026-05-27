// `col` — 12-column grid cell. `span` (1–12, default 12) drives
// `col-span-N`; children render stacked vertically inside.
import { cn } from "@nube/starter-ui-kit";
import { RenderChildren } from "../headless/render.js";
import { registerRenderer } from "../headless/registry.js";

// Static Tailwind class map so the JIT picks the classes up at build
// time — a dynamic `col-span-${n}` string would be tree-shaken away.
const SPAN_CLASSES: Record<number, string> = {
  1: "col-span-1",
  2: "col-span-2",
  3: "col-span-3",
  4: "col-span-4",
  5: "col-span-5",
  6: "col-span-6",
  7: "col-span-7",
  8: "col-span-8",
  9: "col-span-9",
  10: "col-span-10",
  11: "col-span-11",
  12: "col-span-12",
};

export function RenderCol({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const raw = typeof node.span === "number" ? node.span : 12;
  const span = Math.min(12, Math.max(1, Math.round(raw)));
  return (
    <div
      className={cn(
        "sdui-col flex flex-col gap-4",
        SPAN_CLASSES[span],
        node.style?.className,
      )}
    >
      <RenderChildren nodes={node.children} />
    </div>
  );
}

registerRenderer("col", RenderCol);
