// `image` — a standalone content image. `fit`/`aspect` tokens own the
// sizing so authored content never embeds pixel dimensions; an empty
// `alt` renders `alt=""` (decorative, skipped by screen readers).
import { cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";
import { nodeStyleAttrs } from "./node-style.js";

const FIT_CLASS: Record<string, string> = {
  cover: "object-cover",
  contain: "object-contain",
  fill: "object-fill",
};

const ASPECT_CLASS: Record<string, string> = {
  auto: "",
  square: "aspect-square",
  video: "aspect-video",
  wide: "aspect-[21/9]",
  portrait: "aspect-[3/4]",
};

export function RenderImage({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const src = typeof node.src === "string" ? node.src : "";
  const alt = typeof node.alt === "string" ? node.alt : "";
  const fit = FIT_CLASS[node.fit as string] ?? FIT_CLASS.cover;
  const aspect = ASPECT_CLASS[node.aspect as string] ?? ASPECT_CLASS.auto;
  const caption = typeof node.caption === "string" ? node.caption : undefined;
  return (
    <figure
      {...nodeStyleAttrs(node.style)}
      className={cn("sdui-image m-0 flex flex-col gap-2", node.style?.className)}
    >
      <img
        src={src}
        alt={alt}
        className={cn("block w-full overflow-hidden rounded-xl", aspect, fit)}
        loading="lazy"
      />
      {caption ? (
        <figcaption className="text-sm text-[color:var(--color-muted-foreground)]">
          {caption}
        </figcaption>
      ) : null}
    </figure>
  );
}

registerRenderer("image", RenderImage);
