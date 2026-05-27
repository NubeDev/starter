// `page` — root frame. RN port of
// `starter-ui-sdui-react/src/renderer/render-page.tsx`. Mirrors the
// flex-column + optional title structure; styling lives in the kit.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Column, Text, useTheme } from "@nube/starter-ui-kit-native";
import { RenderChildren } from "@nube/starter-ui-sdui-react/headless";
import { registerRenderer } from "@nube/starter-ui-sdui-react/headless";
import { accentHex } from "./accent-colors.js";

export function RenderPage({ node }: { node: UiComponent }) {
  const theme = useTheme();
  const title = typeof node.title === "string" ? node.title : undefined;
  const eyebrow = typeof node.eyebrow === "string" ? node.eyebrow : undefined;
  const leaf = accentHex("leaf", theme.mode);
  return (
    <Column
      padding={16}
      gap={16}
      accessibilityRole="main"
      testID={(node.id as string | undefined) ?? "sdui-page"}
    >
      {eyebrow ? (
        <Text
          variant="caption"
          weight="semibold"
          style={{ color: leaf, letterSpacing: 2.2, textTransform: "uppercase" }}
        >
          {eyebrow}
        </Text>
      ) : null}
      {title ? (
        <Text variant="title" weight="semibold" accessibilityRole="header">
          {title}
        </Text>
      ) : null}
      <RenderChildren nodes={node.children} />
    </Column>
  );
}

registerRenderer("page", RenderPage);
