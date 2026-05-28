import { type NodeProps } from "@xyflow/react";
import { Boxes } from "lucide-react";

type GroupHeaderData = {
  label: string;
  count: number;
};

/// Column heading rendered above each extension swimlane.
///
/// Sized by the layout via `style.width` so the heading visually
/// spans the column. Non-interactive — purely a label.
export function GroupHeaderNode({ data }: NodeProps) {
  const { label, count } = data as GroupHeaderData;
  return (
    <div className="bg-primary/15 border border-primary/30 rounded-md px-3 py-2 flex items-center gap-2 shadow-sm pointer-events-none">
      <Boxes className="h-4 w-4 text-primary shrink-0" />
      <span className="font-semibold text-sm text-foreground tracking-wide truncate">
        {label}
      </span>
      <span className="ml-auto text-xs text-muted-foreground tabular-nums">
        {count} {count === 1 ? "table" : "tables"}
      </span>
    </div>
  );
}
