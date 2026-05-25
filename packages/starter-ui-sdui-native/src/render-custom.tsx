// `custom` — delegates to a host-supplied renderer keyed by
// `node.renderer_id`. Falls back to a tagged "missing renderer" Box
// using the kit's `<Text>` so the placeholder respects theme colour
// and stays accessible to screen readers.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Box, Text } from "@nube/starter-ui-kit-native";
import { registerRenderer, useSduiContext } from "@nube/starter-ui-sdui-react/headless";

export function RenderCustom({ node }: { node: UiComponent }) {
  const id = typeof node.renderer_id === "string" ? node.renderer_id : "";
  const { customRenderers } = useSduiContext();
  const Impl = customRenderers?.[id];
  if (Impl) return <Impl node={node} />;
  return (
    <Box
      padding={8}
      accessibilityLiveRegion="assertive"
      accessibilityLabel={`Missing custom renderer: ${id || "(unset)"}`}
      testID={`sdui-custom-missing-${id || "unset"}`}
    >
      <Text variant="caption" color="destructive">
        Missing custom renderer: {id || "(unset)"}
      </Text>
    </Box>
  );
}

registerRenderer("custom", RenderCustom);
