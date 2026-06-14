import type { Widget, WidgetData } from "@/data/types";
import { Empty } from "@/features/state/Empty";

// Status list — subsystem health. Each row pairs a label column with a
// status column ("online" / "degraded" / "offline", case-insensitive);
// the field mapping's first series carries the status value and its
// `label` field names the label column. Pure DOM, data via props (F6).
// Status → token. Applied as an inline CSS var reference so the colour
// tracks the theme without depending on Tailwind generating an arbitrary
// class for a runtime-computed value.
const DOT_VAR: Record<string, string> = {
  online: "var(--chart-1)",
  degraded: "var(--chart-4)",
  offline: "var(--destructive)",
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
        const dot = DOT_VAR[status] ?? "var(--muted-foreground)";
        return (
          <li
            key={i}
            className="flex items-center justify-between rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-accent/30"
          >
            <span className="truncate text-foreground">{label}</span>
            <span className="flex items-center gap-2">
              <span
                className="size-2 rounded-full"
                style={{
                  backgroundColor: dot,
                  boxShadow: `0 0 8px ${dot}`,
                }}
                aria-hidden
              />
              <span className="capitalize" style={{ color: dot }}>
                {status || "unknown"}
              </span>
            </span>
          </li>
        );
      })}
    </ul>
  );
}
