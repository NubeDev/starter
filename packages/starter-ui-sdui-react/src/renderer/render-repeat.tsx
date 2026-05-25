// `repeat` — duplicates a child template once per row in `node.items`.
// Each iteration sees the row in `page_state["$row"]` so child
// bindings can read it. We don't try to scope per-item state in v1
// (the resolver already inlines repeated trees server-side); this
// is a thin client fallback.
import { cn } from "@nube/starter-ui-kit";
import { Render } from "./render.js";
import { registerRenderer } from "./registry.js";

export function RenderRepeat({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const items = Array.isArray(node.items) ? (node.items as unknown[]) : [];
  const template = node.template as import("@nube/starter-ui-ir").UiComponent | undefined;
  if (!template) return null;
  return (
    <div className={cn("sdui-repeat flex flex-col gap-2", node.style?.className)}>
      {items.map((_item, i) => (
        <Render key={i} node={template} />
      ))}
    </div>
  );
}

registerRenderer("repeat", RenderRepeat);
