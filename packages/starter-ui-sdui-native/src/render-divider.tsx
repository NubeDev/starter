// `divider` — kit-native `<Divider>`.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Divider } from "@nube/starter-ui-kit-native";
import { registerRenderer } from "@nube/starter-ui-sdui-react/headless";

export function RenderDivider({ node }: { node: UiComponent }) {
  const orientation = node.orientation === "vertical" ? "vertical" : "horizontal";
  return (
    <Divider
      orientation={orientation}
      testID={(node.id as string | undefined) ?? "sdui-divider"}
    />
  );
}

registerRenderer("divider", RenderDivider);
