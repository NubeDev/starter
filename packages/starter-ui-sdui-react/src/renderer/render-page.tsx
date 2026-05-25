// `page` — root frame. Renders a flex column with optional title.
import { cn } from "@nube/starter-ui-kit";
import { RenderChildren } from "../headless/render.js";
import { registerRenderer } from "../headless/registry.js";

export function RenderPage({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const title = typeof node.title === "string" ? node.title : undefined;
  return (
    <main className={cn("sdui-page flex flex-col gap-4 p-4", node.style?.className)}>
      {title ? <h1 className="text-2xl font-semibold">{title}</h1> : null}
      <RenderChildren nodes={node.children} />
    </main>
  );
}

registerRenderer("page", RenderPage);
