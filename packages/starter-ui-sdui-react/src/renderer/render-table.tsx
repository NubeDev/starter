// `table` — static table over `node.rows` (server-resolved). The
// paginated `transport.table()` flow is a v2 enhancement; v1 renders
// whatever rows the resolve embedded.
import { cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";

interface Col { key: string; label?: string }

export function RenderTable({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const columns: Col[] = Array.isArray(node.columns)
    ? (node.columns as Col[]).filter((c) => typeof c?.key === "string")
    : [];
  const rows: Record<string, unknown>[] = Array.isArray(node.rows)
    ? (node.rows as Record<string, unknown>[])
    : [];
  return (
    <div className={cn("sdui-table overflow-x-auto", node.style?.className)}>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b text-left">
            {columns.map((c) => (
              <th key={c.key} className="px-2 py-1 font-medium">
                {c.label ?? c.key}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={(row.id as string | undefined) ?? i} className="border-b">
              {columns.map((c) => (
                <td key={c.key} className="px-2 py-1 tabular-nums">
                  {row[c.key] == null ? "" : String(row[c.key])}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

registerRenderer("table", RenderTable);
