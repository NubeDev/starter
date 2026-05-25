// `row` — horizontal flex container. RN port of the web `row`
// renderer. Note: native does not implement Tailwind's 12-column
// grid arithmetic — each child `col` uses `flex` proportional to
// its `span`. See `render-col.tsx`.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Row } from "@nube/starter-ui-kit-native";
import { RenderChildren, registerRenderer } from "@nube/starter-ui-sdui-react/headless";

export function RenderRow({ node }: { node: UiComponent }) {
  return (
    <Row gap={16} wrap testID={(node.id as string | undefined) ?? "sdui-row"}>
      <RenderChildren nodes={node.children} />
    </Row>
  );
}

registerRenderer("row", RenderRow);
