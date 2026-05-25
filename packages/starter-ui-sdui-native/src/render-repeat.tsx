// `repeat` — duplicates a child template once per row in `node.items`.
// v1: thin client fallback — the resolver inlines repeated trees
// server-side in the common case.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Column } from "@nube/starter-ui-kit-native";
import { Render, registerRenderer } from "@nube/starter-ui-sdui-react/headless";
import * as React from "react";

export function RenderRepeat({ node }: { node: UiComponent }) {
  const items = Array.isArray(node.items) ? (node.items as unknown[]) : [];
  const template = node.template as UiComponent | undefined;
  if (!template) return null;
  return (
    <Column gap={8} testID={(node.id as string | undefined) ?? "sdui-repeat"}>
      {items.map((_item, i) => (
        <React.Fragment key={i}>
          <Render node={template} />
        </React.Fragment>
      ))}
    </Column>
  );
}

registerRenderer("repeat", RenderRepeat);
