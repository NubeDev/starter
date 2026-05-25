// `table` — static table over `node.rows`. RN doesn't have a native
// `<table>`; we render as a vertical stack of rows wrapped in
// horizontal `<ScrollArea>` so wide tables remain accessible.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Column, Row, ScrollArea, Text } from "@nube/starter-ui-kit-native";
import { registerRenderer } from "@nube/starter-ui-sdui-react/headless";

interface Col { key: string; label?: string }

export function RenderTable({ node }: { node: UiComponent }) {
  const columns: Col[] = Array.isArray(node.columns)
    ? (node.columns as Col[]).filter((c) => typeof c?.key === "string")
    : [];
  const rows: Record<string, unknown>[] = Array.isArray(node.rows)
    ? (node.rows as Record<string, unknown>[])
    : [];
  return (
    <ScrollArea horizontal testID={(node.id as string | undefined) ?? "sdui-table"}>
      <Column gap={4}>
        <Row gap={12}>
          {columns.map((c) => (
            <Text key={c.key} variant="label" weight="medium">
              {c.label ?? c.key}
            </Text>
          ))}
        </Row>
        {rows.map((row, i) => (
          <Row key={(row.id as string | undefined) ?? i} gap={12}>
            {columns.map((c) => (
              <Text key={c.key} variant="body">
                {row[c.key] == null ? "" : String(row[c.key])}
              </Text>
            ))}
          </Row>
        ))}
      </Column>
    </ScrollArea>
  );
}

registerRenderer("table", RenderTable);
