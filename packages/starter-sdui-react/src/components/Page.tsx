/**
 * `page` — the top-of-tree wrapper. Renders a centred max-width
 * column with an optional title bar; children are projected by the
 * generic `RendererList` and may be any layout / display kind.
 *
 * Shadcn projection: Tailwind utility classes only — no shadcn
 * primitive is needed for a page chrome. The title bar uses a
 * `text-2xl font-semibold` heading + a thin separator.
 */
import type { ComponentSpec } from "../registry/types.js";
import { RendererList } from "../Renderer.js";
import type { UiComponent } from "../types.js";

export interface PageNode extends UiComponent {
  type: "page";
  title?: string;
  children: UiComponent[];
}

export const pageSpec: ComponentSpec<PageNode> = {
  kind: "page",
  Component: ({ node }) => (
    <div className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-6 py-8">
      {node.title ? (
        <div className="flex flex-col gap-3">
          <h1 className="text-2xl font-semibold tracking-tight">{node.title}</h1>
          <div className="h-px w-full bg-border" />
        </div>
      ) : null}
      <RendererList nodes={node.children ?? []} />
    </div>
  ),
};
