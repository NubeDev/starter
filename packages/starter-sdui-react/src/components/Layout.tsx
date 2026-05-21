/**
 * `row` / `col` / `grid` / `stack` — the four primitive layout
 * containers. Each defers to Tailwind flex / grid utilities; the IR
 * declares structure only, the renderer maps to classes.
 *
 * - `row`   — horizontal flex, wraps. Default 12-track behaviour
 *             when a child carries `style.col_span`.
 * - `col`   — vertical flex.
 * - `grid`  — explicit `cols` (1–12) CSS grid.
 * - `stack` — vertical stack with adjustable gap.
 */
import type { ComponentSpec } from "../registry/types.js";
import { RendererList } from "../Renderer.js";
import type { UiComponent } from "../types.js";

type Gap = "xs" | "sm" | "md" | "lg" | "xl";
const GAP: Record<Gap, string> = {
  xs: "gap-1",
  sm: "gap-2",
  md: "gap-4",
  lg: "gap-6",
  xl: "gap-8",
};

export interface RowNode extends UiComponent {
  type: "row";
  gap?: Gap;
  children: UiComponent[];
}
export const rowSpec: ComponentSpec<RowNode> = {
  kind: "row",
  Component: ({ node }) => (
    <div
      className={`flex flex-wrap items-stretch ${GAP[node.gap ?? "md"]} ${
        node.style?.className ?? ""
      }`}
    >
      <RendererList nodes={node.children ?? []} parentId={node.id} parentType="row" />
    </div>
  ),
};

export interface ColNode extends UiComponent {
  type: "col";
  gap?: Gap;
  span?: number;
  children: UiComponent[];
}
export const colSpec: ComponentSpec<ColNode> = {
  kind: "col",
  Component: ({ node }) => {
    const span = node.span;
    const style =
      span !== undefined ? { flex: `0 0 ${(span / 12) * 100}%` } : undefined;
    return (
      <div
        style={style}
        className={`flex flex-col ${GAP[node.gap ?? "md"]} ${
          node.style?.className ?? ""
        }`}
      >
        <RendererList nodes={node.children ?? []} parentId={node.id} parentType="col" />
      </div>
    );
  },
};

export interface GridNode extends UiComponent {
  type: "grid";
  cols?: number;
  gap?: Gap;
  children: UiComponent[];
}
const GRID_COLS: Record<number, string> = {
  1: "grid-cols-1",
  2: "grid-cols-1 sm:grid-cols-2",
  3: "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3",
  4: "grid-cols-1 sm:grid-cols-2 lg:grid-cols-4",
  6: "grid-cols-2 sm:grid-cols-3 lg:grid-cols-6",
  12: "grid-cols-12",
};
export const gridSpec: ComponentSpec<GridNode> = {
  kind: "grid",
  Component: ({ node }) => (
    <div
      className={`grid ${GRID_COLS[node.cols ?? 4] ?? GRID_COLS[4]} ${
        GAP[node.gap ?? "md"]
      } ${node.style?.className ?? ""}`}
    >
      <RendererList nodes={node.children ?? []} parentId={node.id} parentType="grid" />
    </div>
  ),
};

export interface StackNode extends UiComponent {
  type: "stack";
  gap?: Gap;
  children: UiComponent[];
}
export const stackSpec: ComponentSpec<StackNode> = {
  kind: "stack",
  Component: ({ node }) => (
    <div
      className={`flex flex-col ${GAP[node.gap ?? "md"]} ${
        node.style?.className ?? ""
      }`}
    >
      <RendererList nodes={node.children ?? []} parentId={node.id} parentType="stack" />
    </div>
  ),
};
