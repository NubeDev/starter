// Shared NodeStyle → presentation mapping.
//
// The SDUI IR (`crates/starter-ui-ir`) lets any node carry a `NodeStyle`
// triplet-plus: `intent`, `density`, `surface`, `radius`, `spacing`,
// plus the richer page-builder tokens (`background`, `gradient`,
// `shadow`, `text_align`, `font_size`, `font_weight`). Historically the
// React renderer ignored all of these except the undocumented
// `className` escape hatch, which is *why* authored pages all looked
// identical ("grafana-like boring widgets"): the IR described rich
// styling but nothing painted it.
//
// This helper turns the closed-set style tokens into stable `data-sdui-*`
// attributes. The matching CSS lives in `styles/node-style.css` (a static
// stylesheet, so Tailwind tree-shaking can never drop it) and resolves
// every token to a theme variable from `@nube/starter-ui-kit`. No raw
// hex / pixel values ever appear — authors pick tokens, the theme decides
// the actual colour, keeping light/dark consistent.

export interface NodeStyleLike {
  intent?: string;
  density?: string;
  surface?: string;
  radius?: string;
  spacing?: string;
  // Page-builder decoration tokens (V2.0):
  background?: string;
  gradient?: string;
  shadow?: string;
  text_align?: string;
  font_size?: string;
  font_weight?: string;
  align?: string;
  className?: string;
}

// Closed token sets — anything outside these is dropped so a malformed
// or hostile tree can never inject an arbitrary attribute selector.
const SURFACE = new Set(["default", "raised", "subtle", "transparent"]);
const RADIUS = new Set(["none", "sm", "md", "lg", "xl", "full"]);
const SPACING = new Set(["none", "xs", "sm", "md", "lg", "xl", "2xl"]);
const INTENT = new Set(["info", "success", "warning", "danger", "muted"]);
const DENSITY = new Set(["compact", "normal", "comfortable"]);
const BACKGROUND = new Set([
  "none",
  "surface",
  "muted",
  "subtle",
  "leaf",
  "aqua",
  "sun",
  "sky",
  "warn",
  "ink",
]);
const GRADIENT = new Set([
  "none",
  "leaf",
  "aqua",
  "sun",
  "sky",
  "dusk",
  "ink",
]);
const SHADOW = new Set(["none", "sm", "md", "lg", "xl", "glow"]);
const TEXT_ALIGN = new Set(["start", "center", "end"]);
const FONT_SIZE = new Set(["xs", "sm", "md", "lg", "xl", "2xl", "3xl", "4xl"]);
const FONT_WEIGHT = new Set(["normal", "medium", "semibold", "bold"]);

function pick(value: unknown, allowed: Set<string>): string | undefined {
  return typeof value === "string" && allowed.has(value) ? value : undefined;
}

/**
 * Build the `data-sdui-*` attribute bag for a node's style. Spread onto
 * the node's outermost element:
 *
 *   <section {...nodeStyleAttrs(node.style)} className={cn("...", node.style?.className)}>
 *
 * Returns `{}` when `style` is absent so it is always safe to spread.
 */
export function nodeStyleAttrs(
  style: NodeStyleLike | undefined | null,
): Record<string, string> {
  if (!style || typeof style !== "object") return {};
  const attrs: Record<string, string> = {};
  const set = (key: string, v: string | undefined) => {
    if (v) attrs[key] = v;
  };
  set("data-sdui-intent", pick(style.intent, INTENT));
  set("data-sdui-density", pick(style.density, DENSITY));
  set("data-sdui-surface", pick(style.surface, SURFACE));
  set("data-sdui-radius", pick(style.radius, RADIUS));
  set("data-sdui-spacing", pick(style.spacing, SPACING));
  set("data-sdui-background", pick(style.background, BACKGROUND));
  set("data-sdui-gradient", pick(style.gradient, GRADIENT));
  set("data-sdui-shadow", pick(style.shadow, SHADOW));
  set("data-sdui-text-align", pick(style.text_align, TEXT_ALIGN));
  set("data-sdui-font-size", pick(style.font_size, FONT_SIZE));
  set("data-sdui-font-weight", pick(style.font_weight, FONT_WEIGHT));
  return attrs;
}
