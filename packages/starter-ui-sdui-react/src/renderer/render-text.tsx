// `text` — a plain text block. Honours typography + alignment tokens
// from `style` (font_size / font_weight / text_align), so it doubles as
// the editorial body-copy primitive in a page-builder layout.
import { cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";
import { nodeStyleAttrs } from "./node-style.js";

const INTENT_VAR: Record<string, string> = {
  info: "var(--color-sky)",
  success: "var(--color-leaf)",
  warning: "var(--color-warn)",
  danger: "var(--color-danger)",
  muted: "var(--color-muted-foreground)",
};

export function RenderText({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const content = typeof node.content === "string" ? node.content : "";
  const color = typeof node.intent === "string" ? INTENT_VAR[node.intent] : undefined;
  return (
    <p
      {...nodeStyleAttrs(node.style)}
      className={cn("sdui-text leading-relaxed text-[color:var(--color-text)]", node.style?.className)}
      style={color ? { color } : undefined}
    >
      {content}
    </p>
  );
}

registerRenderer("text", RenderText);
