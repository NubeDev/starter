// `page` — root frame. RN port of
// `starter-ui-sdui-react/src/renderer/render-page.tsx`. Mirrors the
// flex-column + optional title structure; styling lives in the kit.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Column, Text } from "@nube/starter-ui-kit-native";
import { RenderChildren } from "@nube/starter-ui-sdui-react/headless";
import { registerRenderer } from "@nube/starter-ui-sdui-react/headless";

export function RenderPage({ node }: { node: UiComponent }) {
  const title = typeof node.title === "string" ? node.title : undefined;
  return (
    <Column
      padding={16}
      gap={16}
      testID={(node.id as string | undefined) ?? "sdui-page"}
    >
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
