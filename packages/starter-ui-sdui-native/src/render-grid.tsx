// `grid` — RN does not have CSS grid; we lay children out as a
// wrapping row where each cell takes `(100 / columns)%` of the row.
// Per spec, `kpi_grid` is in the deferred-with-web set and is NOT
// aliased here.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Box, Row } from "@nube/starter-ui-kit-native";
import { RenderChildren, registerRenderer } from "@nube/starter-ui-sdui-react/headless";

export function RenderGrid({ node }: { node: UiComponent }) {
  const cols = typeof node.columns === "number" && node.columns > 0 ? node.columns : 3;
  const basis = `${100 / cols}%`;
  // Wrap each child so the basis applies uniformly without forcing
  // the renderer files for `row`/`col` to know about grid layout.
  const children = Array.isArray(node.children) ? node.children : [];
  return (
    <Row
      gap={12}
      wrap
      testID={(node.id as string | undefined) ?? "sdui-grid"}
    >
      {children.map((c, i) => (
        <Box
          key={(c.id as string | undefined) ?? `${c.type}:${i}`}
          style={{ flexBasis: basis as unknown as number }}
        >
          <RenderChildren nodes={[c]} />
        </Box>
      ))}
    </Row>
  );
}

registerRenderer("grid", RenderGrid);
