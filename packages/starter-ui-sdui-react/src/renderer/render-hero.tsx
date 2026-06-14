// `hero` — a full-width feature band with editorial defaults: an
// optional eyebrow, a large centred title, a subtitle, and a body
// subtree (typically CTA buttons / badges). Visual background comes from
// the node's `style` (`gradient`/`background`/`shadow`); the author drops
// a hero, picks `gradient: dusk`, and gets a banner without touching CSS.
import { cn } from "@nube/starter-ui-kit";
import { RenderChildren } from "../headless/render.js";
import { registerRenderer } from "../headless/registry.js";
import { nodeStyleAttrs } from "./node-style.js";

export function RenderHero({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const eyebrow = typeof node.eyebrow === "string" ? node.eyebrow : undefined;
  const title = typeof node.title === "string" ? node.title : "";
  const subtitle = typeof node.subtitle === "string" ? node.subtitle : undefined;
  const hasBody = Array.isArray(node.children) && node.children.length > 0;
  return (
    <section
      {...nodeStyleAttrs(node.style)}
      className={cn(
        // Editorial defaults — overridable by style tokens (spacing,
        // radius, text-align, font-size all win over these via CSS
        // attribute selectors having higher specificity than utilities
        // is NOT guaranteed, so we keep utilities to layout only).
        "sdui-hero flex flex-col items-center gap-4 rounded-2xl px-6 py-12 text-center sm:px-12 sm:py-16",
        node.style?.className,
      )}
    >
      {eyebrow ? (
        <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
          {eyebrow}
        </span>
      ) : null}
      <h2 className="max-w-3xl text-3xl font-semibold leading-[1.1] tracking-[-0.02em] text-[color:var(--color-text)] sm:text-5xl">
        {title}
      </h2>
      {subtitle ? (
        <p className="max-w-2xl text-base text-[color:var(--color-muted-foreground)] sm:text-lg">
          {subtitle}
        </p>
      ) : null}
      {hasBody ? (
        <div className="mt-2 flex flex-wrap items-center justify-center gap-3">
          <RenderChildren nodes={node.children} />
        </div>
      ) : null}
    </section>
  );
}

registerRenderer("hero", RenderHero);
