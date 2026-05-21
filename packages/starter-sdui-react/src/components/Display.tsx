/**
 * Display primitives: `text`, `heading`, `badge`.
 *
 * `text` projects against `<p>` with optional tone (`muted` /
 * `danger` / `success`). `heading` picks an `h1`–`h6` via `level`.
 * `badge` projects against the shadcn Badge primitive.
 */
import { Badge } from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import type { UiComponent } from "../types.js";

type Tone = "default" | "muted" | "danger" | "success" | "warning";
const TONE_TEXT: Record<Tone, string> = {
  default: "text-foreground",
  muted: "text-muted-foreground",
  danger: "text-destructive",
  success: "text-emerald-600",
  warning: "text-amber-600",
};

export interface TextNode extends UiComponent {
  type: "text";
  value: string;
  tone?: Tone;
}
export const textSpec: ComponentSpec<TextNode> = {
  kind: "text",
  Component: ({ node }) => (
    <p className={`text-sm ${TONE_TEXT[node.tone ?? "default"]} ${node.style?.className ?? ""}`}>
      {node.value}
    </p>
  ),
};

export interface HeadingNode extends UiComponent {
  type: "heading";
  value: string;
  level?: 1 | 2 | 3 | 4 | 5 | 6;
}
const HEADING_CLASS: Record<number, string> = {
  1: "text-3xl font-semibold tracking-tight",
  2: "text-2xl font-semibold tracking-tight",
  3: "text-xl font-semibold",
  4: "text-lg font-semibold",
  5: "text-base font-semibold",
  6: "text-sm font-semibold",
};
export const headingSpec: ComponentSpec<HeadingNode> = {
  kind: "heading",
  Component: ({ node }) => {
    const level = node.level ?? 2;
    const Tag = (`h${level}` as unknown) as keyof JSX.IntrinsicElements;
    return (
      <Tag className={`${HEADING_CLASS[level]} ${node.style?.className ?? ""}`}>
        {node.value}
      </Tag>
    );
  },
};

type BadgeVariant = "default" | "secondary" | "destructive" | "outline";
export interface BadgeNode extends UiComponent {
  type: "badge";
  label: string;
  variant?: BadgeVariant;
}
export const badgeSpec: ComponentSpec<BadgeNode> = {
  kind: "badge",
  Component: ({ node }) => (
    <Badge variant={node.variant ?? "default"} className={node.style?.className}>
      {node.label}
    </Badge>
  ),
};
