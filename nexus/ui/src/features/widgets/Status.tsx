import type { Widget, WidgetData } from "@/data/types";
import { Empty } from "@/features/state/Empty";

// Status list — subsystem health. Each row pairs a label column with a
// status column ("online" / "degraded" / "offline", case-insensitive);
// the field mapping's first series carries the status value and its
// `label` field names the label column. Pure DOM, data via props (F6).
const DOT: Record<string, string> = {
  online: "bg-[hsl(var(--primary))]",
  degraded: "bg-[hsl(var(--chart-4))]",
  offline: "bg-[hsl(var(--destructive))]",
};

export function Status({ widget, data }: { widget: Widget; data: WidgetData }) {
  const statusField = widget.config.fields.series[0];
  const labelCol = widget.config.fields.x;
  if (!statusField || data.points.length === 0) {
    return <Empty title={widget.title} description="No data" />;
  }

  return (
    <ul className="flex h-full flex-col gap-1 overflow-y-auto">
      {data.points.map((p, i) => {
        const status = String(p[statusField.value] ?? "").toLowerCase();
        const label = labelCol ? String(p[labelCol] ?? "—") : `#${i + 1}`;
        return (
          <li
            key={i}
            className="flex items-center justify-between rounded-md px-2 py-1.5 text-sm"
          >
            <span className="truncate text-foreground">{label}</span>
            <span className="flex items-center gap-2 text-muted-foreground">
              <span
                className={`size-2 rounded-full ${DOT[status] ?? "bg-muted"}`}
                aria-hidden
              />
              <span className="capitalize">{status || "unknown"}</span>
            </span>
          </li>
        );
      })}
    </ul>
  );
}
