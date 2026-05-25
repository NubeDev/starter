// Central walker — takes one IR node and dispatches into the
// registry. Unknown variants render a "Dangling" placeholder so an
// IR addition the client doesn't know about degrades gracefully.

import type { UiComponent } from "@nube/starter-ui-ir";
import { lookupRenderer } from "./registry.js";

export function Render({ node }: { node: UiComponent }) {
  const R = lookupRenderer(node.type);
  if (R) return <R node={node} />;
  return (
    <div
      data-sdui-dangling={node.type}
      style={{
        padding: 8,
        border: "1px dashed #c33",
        color: "#c33",
        fontSize: 12,
      }}
    >
      Unknown component: <code>{node.type}</code>
    </div>
  );
}

/** Render a children[] tuple. Stable keys via `id ?? index`. */
export function RenderChildren({ nodes }: { nodes?: UiComponent[] }) {
  if (!nodes || nodes.length === 0) return null;
  return (
    <>
      {nodes.map((c, i) => (
        <Render key={c.id ?? `${c.type}:${i}`} node={c} />
      ))}
    </>
  );
}
