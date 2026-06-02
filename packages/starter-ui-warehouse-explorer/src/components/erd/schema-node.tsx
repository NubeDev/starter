// Reference-style ERD node for the new Schema Explorer page.
//
// Distinct from the legacy `table-node.tsx` (kept untouched for the old
// `Schema` view). This node mirrors the polished schema-viewer look:
//   - a quiet header with a table/view glyph + name, and a `VIEW` chip
//     for views,
//   - a NAME / TYPE / NULL column grid instead of one dense line,
//   - key glyphs for primary / foreign keys,
//   - a selection ring driven by `selected` so it stays in sync with the
//     left tree.
//
// Connection handles are kept but visually hidden — edges attach per
// column, the dots themselves add noise the reference deliberately avoids.

import { Handle, Position, type NodeProps } from "@xyflow/react";
import { Eye, KeyRound, Link2, Table2 } from "lucide-react";

import { cn } from "../../lib/utils";

export type SchemaColumn = {
  name: string;
  data_type: string;
  nullable: boolean;
  is_primary_key: boolean;
  /** Set by the canvas when a relationship originates from this column. */
  is_foreign_key?: boolean;
};

export type SchemaNodeData = {
  label: string;
  columns: SchemaColumn[];
  kind: "table" | "view";
  /** Names of columns participating in any relationship — get a handle dot. */
  connected?: Set<string>;
};

export function SchemaNode({ data, selected }: NodeProps) {
  const { label, columns, kind, connected } = data as SchemaNodeData;
  const isView = kind === "view";

  return (
    <div
      className={cn(
        "min-w-[260px] overflow-hidden rounded-xl border bg-card text-card-foreground shadow-sm transition-shadow",
        "hover:shadow-md",
        selected
          ? "border-primary ring-2 ring-primary/40"
          : "border-border",
      )}
    >
      {/* Header */}
      <div className="flex items-center gap-2 border-b border-border bg-muted/40 px-3 py-2.5">
        {isView ? (
          <Eye className="h-4 w-4 shrink-0 text-muted-foreground" />
        ) : (
          <Table2 className="h-4 w-4 shrink-0 text-muted-foreground" />
        )}
        <span className="truncate font-mono text-[13px] font-semibold tracking-tight text-foreground">
          {label}
        </span>
        {isView && (
          <span className="ml-auto rounded bg-secondary px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-muted-foreground">
            View
          </span>
        )}
      </div>

      {/* Column header row */}
      <div className="grid grid-cols-[1fr_auto_2.25rem] items-center gap-3 border-b border-border/60 px-3 py-1.5 text-[9px] font-semibold uppercase tracking-wider text-muted-foreground/70">
        <span>Name</span>
        <span className="text-right">Type</span>
        <span className="text-right">Null</span>
      </div>

      {/* Columns */}
      <div>
        {columns.map((column) => {
          const hasHandle = connected?.has(column.name) ?? false;
          return (
            <div
              key={column.name}
              className={cn(
                "relative grid grid-cols-[1fr_auto_2.25rem] items-center gap-3 px-3 py-1.5 text-[12px]",
                "border-b border-border/40 last:border-b-0",
                column.is_primary_key && "bg-primary/[0.04]",
              )}
            >
              {hasHandle && (
                <>
                  <Handle
                    type="target"
                    position={Position.Left}
                    id={column.name}
                    className="!h-1.5 !w-1.5 !border-0 !bg-primary/70"
                  />
                  <Handle
                    type="source"
                    position={Position.Right}
                    id={column.name}
                    className="!h-1.5 !w-1.5 !border-0 !bg-primary/70"
                  />
                </>
              )}

              <span className="flex min-w-0 items-center gap-1.5">
                {column.is_primary_key ? (
                  <KeyRound className="h-3 w-3 shrink-0 text-amber-500" />
                ) : column.is_foreign_key ? (
                  <Link2 className="h-3 w-3 shrink-0 text-sky-500" />
                ) : (
                  <span className="w-3 shrink-0" />
                )}
                <span
                  className={cn(
                    "truncate font-mono",
                    column.is_primary_key
                      ? "font-medium text-foreground"
                      : "text-foreground/90",
                  )}
                >
                  {column.name}
                </span>
              </span>

              <span className="truncate text-right font-mono text-[11px] text-muted-foreground">
                {column.data_type}
              </span>

              <span className="text-right font-mono text-[10px] text-muted-foreground/80">
                {column.nullable ? "Yes" : "No"}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
