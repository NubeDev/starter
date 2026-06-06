// `card` — titled container with optional lead/trailing slots, header
// actions, and a body subtree. A surface by default (raised), but every
// visual is overridable via `style` tokens (background/gradient/radius/
// shadow/spacing). The richer cousin of a bare `col`.
import { cn } from "@nube/starter-ui-kit";
import { Render, RenderChildren } from "../headless/render.js";
import { registerRenderer } from "../headless/registry.js";
import { nodeStyleAttrs } from "./node-style.js";

const INTENT_ACCENT: Record<string, string> = {
  info: "var(--color-sky)",
  success: "var(--color-leaf)",
  warning: "var(--color-warn)",
  danger: "var(--color-danger)",
};

export function RenderCard({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const title = typeof node.title === "string" ? node.title : undefined;
  const subtitle = typeof node.subtitle === "string" ? node.subtitle : undefined;
  const intent = typeof node.intent === "string" ? INTENT_ACCENT[node.intent] : undefined;
  const lead = node.lead as import("@nube/starter-ui-ir").UiComponent | undefined;
  const trailing = node.trailing as import("@nube/starter-ui-ir").UiComponent | undefined;
  const hasHeader = title || lead || trailing;
  return (
    <div
      {...nodeStyleAttrs(node.style)}
      className={cn(
        "sdui-card flex flex-col gap-4 rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] p-5",
        node.style?.className,
      )}
      style={intent ? { borderInlineStartWidth: 3, borderInlineStartColor: intent } : undefined}
    >
      {hasHeader ? (
        <header className="flex items-start gap-3">
          {lead ? <div className="shrink-0">{<Render node={lead} />}</div> : null}
          <div className="flex flex-1 flex-col gap-0.5">
            {title ? (
              <h3 className="text-base font-semibold leading-tight text-[color:var(--color-text)]">
                {title}
              </h3>
            ) : null}
            {subtitle ? (
              <p className="text-sm text-[color:var(--color-muted-foreground)]">{subtitle}</p>
            ) : null}
          </div>
          {trailing ? <div className="shrink-0">{<Render node={trailing} />}</div> : null}
        </header>
      ) : null}
      <RenderChildren nodes={node.children} />
    </div>
  );
}

registerRenderer("card", RenderCard);
