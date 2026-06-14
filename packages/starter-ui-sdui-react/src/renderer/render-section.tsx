// `section` — a titled content band. Heading + optional subtitle over an
// arbitrary child subtree. The visual background/padding/shadow come from
// the node's `style` tokens (see `node-style.ts`), so a `section` is the
// primary "make this stretch of the page stand out" container.
import { cn } from "@nube/starter-ui-kit";
import { RenderChildren } from "../headless/render.js";
import { registerRenderer } from "../headless/registry.js";
import { nodeStyleAttrs } from "./node-style.js";

const LANDMARK_TAG: Record<string, "section" | "nav" | "aside" | "form"> = {
  region: "section",
  nav: "nav",
  complementary: "aside",
  form: "form",
};

export function RenderSection({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const title = typeof node.title === "string" ? node.title : undefined;
  const subtitle = typeof node.subtitle === "string" ? node.subtitle : undefined;
  const level = Math.min(6, Math.max(2, Math.round(Number(node.level) || 3)));
  const Heading = `h${level}` as "h2";
  const landmark =
    typeof node.landmark === "string" ? LANDMARK_TAG[node.landmark] : undefined;
  const Tag = landmark ?? "section";
  return (
    <Tag
      {...nodeStyleAttrs(node.style)}
      className={cn("sdui-section flex flex-col gap-4", node.style?.className)}
    >
      {title ? (
        <header className="flex flex-col gap-1">
          <Heading className="text-lg font-semibold leading-tight text-[color:var(--color-text)]">
            {title}
          </Heading>
          {subtitle ? (
            <p className="text-sm text-[color:var(--color-muted-foreground)]">{subtitle}</p>
          ) : null}
        </header>
      ) : null}
      <RenderChildren nodes={node.children} />
    </Tag>
  );
}

registerRenderer("section", RenderSection);
