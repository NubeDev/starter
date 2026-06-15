// Lean client-side renderer for the declarative UI DSL (src/lib/ui/types.ts).
// A registry maps a widget `type` → React component; a walker renders a node and
// recurses. Unknown types degrade to a visible placeholder (never throw). This
// reuses SDUI's registry *idea* but binds straight to the live value store —
// no server resolve. See ../../SDUI_UNIFIED_DESIGN.md §10.

import type { ComponentType } from "react";
import type { Widget } from "../lib/ui/types";
import type { FlexValue } from "../lib/engine-types";

/** Context threaded to every widget while rendering a view. */
export interface RenderCtx {
  /** the component this view is bound to (follow/sync selection), if any */
  componentUid?: number;
  /** invoke a component action (`POST /call/nodes/uid/{uid}`) → its `returns` */
  callAction?: (
    componentUid: number,
    name: string,
    params?: Record<string, FlexValue>,
  ) => Promise<Record<string, FlexValue>>;
}

export interface WidgetProps {
  node: Widget;
  ctx: RenderCtx;
}

const REGISTRY: Record<string, ComponentType<WidgetProps>> = {};

export function registerWidget(type: string, comp: ComponentType<WidgetProps>): void {
  REGISTRY[type] = comp;
}

export function lookupWidget(type: string): ComponentType<WidgetProps> | undefined {
  return REGISTRY[type];
}

export function listWidgets(): string[] {
  return Object.keys(REGISTRY);
}

/** Render one widget node by looking up its `type` in the registry. */
export function RenderWidget({ node, ctx }: WidgetProps) {
  const C = lookupWidget(node.type);
  if (!C) {
    return (
      <div
        style={{
          border: "1px dashed #c0392b",
          color: "#c0392b",
          padding: "4px 8px",
          fontSize: 11,
          borderRadius: 3,
        }}
      >
        unknown widget: {node.type}
      </div>
    );
  }
  return <C node={node} ctx={ctx} />;
}
