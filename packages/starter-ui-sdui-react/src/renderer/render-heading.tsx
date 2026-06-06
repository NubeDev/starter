// `heading` — an h1–h6 heading with optional subtitle. Defaults to a
// sensible size per level; `style` typography tokens (font_size etc.)
// override when the author wants an oversized editorial headline.
import { cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";
import { nodeStyleAttrs } from "./node-style.js";

const LEVEL_CLASS: Record<number, string> = {
  1: "text-4xl",
  2: "text-3xl",
  3: "text-2xl",
  4: "text-xl",
  5: "text-lg",
  6: "text-base",
};

export function RenderHeading({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const content = typeof node.content === "string" ? node.content : "";
  const subtitle = typeof node.subtitle === "string" ? node.subtitle : undefined;
  const level = Math.min(6, Math.max(1, Math.round(Number(node.level) || 2)));
  const Tag = `h${level}` as "h2";
  return (
    <div
      {...nodeStyleAttrs(node.style)}
      className={cn("sdui-heading flex flex-col gap-1", node.style?.className)}
    >
      <Tag
        className={cn(
          "font-semibold leading-tight tracking-[-0.01em] text-[color:var(--color-text)]",
          LEVEL_CLASS[level],
        )}
      >
        {content}
      </Tag>
      {subtitle ? (
        <p className="text-sm text-[color:var(--color-muted-foreground)]">{subtitle}</p>
      ) : null}
    </div>
  );
}

registerRenderer("heading", RenderHeading);
