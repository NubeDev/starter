/**
 * `table` — projects a `UiTable` (rows + columns) against raw HTML
 * `<table>` styled with shadcn-aligned utility classes (no shadcn
 * Table primitive in starter-ui-kit yet).
 *
 * Two modes:
 *   - `columns` — each column declares `{ key, label }` and renders
 *     `row.slots[key]` as text.
 *   - `row_template` — every row is projected through the renderer
 *     against a tree template; `{{$row.*}}` tokens are substituted
 *     via `bindRow` from `row-bind.ts`.
 *
 * Server-side row queries (`/api/v1/ui/table`) happen one level up;
 * the IR carries the resolved row set as `rows`.
 */
import type { ComponentSpec } from "../registry/types.js";
import { Renderer } from "../Renderer.js";
import { bindRow } from "../row-bind.js";
import type { UiComponent, UiTableRow } from "../types.js";

export interface TableColumn {
  key: string;
  label: string;
}

export interface TableNode extends UiComponent {
  type: "table";
  rows: UiTableRow[];
  columns?: TableColumn[];
  row_template?: UiComponent;
  empty_message?: string;
}

export const tableSpec: ComponentSpec<TableNode> = {
  kind: "table",
  Component: ({ node }) => {
    const rows = node.rows ?? [];

    if (rows.length === 0) {
      return (
        <div className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
          {node.empty_message ?? "No rows."}
        </div>
      );
    }

    if (node.row_template) {
      const tpl = node.row_template;
      return (
        <div className="flex flex-col gap-2">
          {rows.map((r) => (
            <Renderer key={r.id} node={bindRow(tpl, r)} />
          ))}
        </div>
      );
    }

    const columns: TableColumn[] = node.columns ?? [];

    return (
      <div className="w-full overflow-auto rounded-md border">
        <table className="w-full caption-bottom text-sm">
          <thead className="border-b bg-muted/50">
            <tr>
              {columns.map((c) => (
                <th
                  key={c.key}
                  className="h-10 px-3 text-left align-middle font-medium text-muted-foreground"
                >
                  {c.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr key={r.id} className="border-b transition-colors hover:bg-muted/30">
                {columns.map((c) => (
                  <td key={c.key} className="px-3 py-2 align-middle">
                    {formatCell(r.slots[c.key])}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  },
};

function formatCell(value: unknown): string {
  if (value == null) return "";
  if (typeof value === "string" || typeof value === "number") return String(value);
  if (typeof value === "boolean") return value ? "yes" : "no";
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}
