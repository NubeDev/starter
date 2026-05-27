// `page` — root frame. Renders a flex column with optional title.
import { cn } from "@nube/starter-ui-kit";
import { RenderChildren } from "../headless/render.js";
import { registerRenderer } from "../headless/registry.js";

export function RenderPage({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const title = typeof node.title === "string" ? node.title : undefined;
  const eyebrow = typeof node.eyebrow === "string" ? node.eyebrow : undefined;
  return (
    <main className={cn("sdui-page flex flex-col gap-6 p-4 sm:p-6", node.style?.className)}>
      {title ? (
        <header className="sdui-page-header flex flex-col gap-2">
          {eyebrow ? (
            <div className="flex items-center gap-3">
              <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
              <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
                {eyebrow}
              </span>
            </div>
          ) : null}
          <h1 className="text-3xl font-medium leading-[1.1] tracking-[-0.02em] text-[color:var(--color-text)] sm:text-4xl">
            {title}
          </h1>
        </header>
      ) : null}
      <RenderChildren nodes={node.children} />
    </main>
  );
}

registerRenderer("page", RenderPage);
