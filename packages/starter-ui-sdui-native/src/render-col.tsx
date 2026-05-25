// `col` — flex cell. `span` (1–12, default 12) maps to a `flex`
// factor so the row distributes width proportionally.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Column } from "@nube/starter-ui-kit-native";
import { RenderChildren, registerRenderer } from "@nube/starter-ui-sdui-react/headless";

export function RenderCol({ node }: { node: UiComponent }) {
  const raw = typeof node.span === "number" ? node.span : 12;
  const span = Math.min(12, Math.max(1, Math.round(raw)));
  // `flexBasis: 0` keeps the proportional distribution honest when the
  // row is wide enough to satisfy intrinsic content sizes.
  return (
    <Column
      gap={12}
      flex={span}
      style={{ flexBasis: 0, minWidth: 0 }}
      testID={(node.id as string | undefined) ?? "sdui-col"}
    >
      <RenderChildren nodes={node.children} />
    </Column>
  );
}

registerRenderer("col", RenderCol);
