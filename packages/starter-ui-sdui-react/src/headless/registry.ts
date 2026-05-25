// Variant → React component dispatcher. Each renderer file in
// `./` registers itself by exporting a default + a `variant` string;
// the central `Render` walker reads this map.

import type { ComponentType } from "react";
import type { UiComponent } from "@nube/starter-ui-ir";

export interface RenderProps<C extends UiComponent = UiComponent> {
  node: C;
}

export type Renderer = ComponentType<RenderProps>;

/** Custom renderers (rubix-supplied) — keyed by `node.renderer_id`. */
export type CustomRendererRegistry = Record<string, Renderer>;

/** Built-in dispatch map, keyed by `node.type`. */
const REGISTRY: Record<string, Renderer> = {};

export function registerRenderer(variant: string, render: Renderer): void {
  REGISTRY[variant] = render;
}

export function lookupRenderer(variant: string): Renderer | undefined {
  return REGISTRY[variant];
}

export function listRenderers(): string[] {
  return Object.keys(REGISTRY).sort();
}
