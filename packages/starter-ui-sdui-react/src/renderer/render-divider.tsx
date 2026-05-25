// `divider` — `<Separator>` from the UI kit.
import { Separator, cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "./registry.js";

export function RenderDivider({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const orientation = node.orientation === "vertical" ? "vertical" : "horizontal";
  return (
    <Separator
      orientation={orientation}
      className={cn("sdui-divider", node.style?.className)}
    />
  );
}

registerRenderer("divider", RenderDivider);
