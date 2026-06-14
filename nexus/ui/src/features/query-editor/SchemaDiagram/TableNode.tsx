import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { KeyRound, Table2 } from "lucide-react";

import type { DiagramNode } from "@/features/query-editor/SchemaDiagram/layout";

// One table rendered as an ER card: a header with the (schema-qualified) name
// and a column list, FK columns marked with a key icon. A single pair of
// handles (left target / right source) carries every FK edge in/out of the
// table — React Flow routes them, so individual columns don't need their own
// handles for a readable diagram at this scale.
function TableNodeCard({ data }: NodeProps) {
  const d = data as unknown as DiagramNode;
  const qualified = d.schema === "public" ? d.name : `${d.schema}.${d.name}`;
  return (
    <div className="glass min-w-56 max-w-72 overflow-hidden rounded-lg border border-border/70">
      <Handle type="target" position={Position.Left} className="!bg-primary" />
      <div className="flex items-center gap-1.5 border-b border-border/60 bg-primary/5 px-2.5 py-1.5">
        <Table2 className="size-3.5 shrink-0 text-primary" />
        <span className="truncate text-sm font-semibold text-foreground" title={qualified}>
          {qualified}
        </span>
        <span className="ms-auto shrink-0 text-[10px] tabular-nums text-muted-foreground">
          {d.table.columns.length}
        </span>
      </div>
      <div className="max-h-56 overflow-y-auto py-1">
        {d.table.columns.length === 0 ? (
          <p className="px-2.5 py-1 text-xs italic text-muted-foreground/60">
            no columns
          </p>
        ) : (
          d.table.columns.map((c) => {
            const isFk = d.fkColumns.has(c.name);
            return (
              <div
                key={c.name}
                className="flex items-center gap-1.5 px-2.5 py-0.5 text-xs"
              >
                {isFk ? (
                  <KeyRound className="size-3 shrink-0 text-amber-500" />
                ) : (
                  <span className="inline-block size-3 shrink-0" />
                )}
                <span
                  className={
                    isFk
                      ? "truncate font-medium text-foreground"
                      : "truncate text-muted-foreground"
                  }
                >
                  {c.name}
                </span>
                <span className="ms-auto shrink-0 font-mono text-[0.65rem] text-muted-foreground/50">
                  {c.data_type}
                </span>
              </div>
            );
          })
        )}
      </div>
      <Handle type="source" position={Position.Right} className="!bg-primary" />
    </div>
  );
}

export const TableNode = memo(TableNodeCard);
