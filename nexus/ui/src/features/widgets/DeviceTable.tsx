import type { Widget, WidgetData } from "@/data/types";
import { Empty } from "@/features/state/Empty";
import { useDateTime } from "@/datetime";

// Device / row table. Columns come from the field mapping: the `x`
// column (if any) is the leading key column, followed by each mapped
// series. Renders the raw query rows — no client-side aggregation, no
// fabricated rows (F0/F6). Numeric series use tabular figures; columns
// declared `kind: "time"` render through the active region/preference
// date formatter.
export function DeviceTable({
  widget,
  data,
}: {
  widget: Widget;
  data: WidgetData;
}) {
  const { x, xKind, series } = widget.config.fields;
  const { dateTime } = useDateTime();
  if (data.points.length === 0) {
    return <Empty title={widget.title} description="No rows" />;
  }

  const columns = [
    ...(x ? [{ key: x, label: x, numeric: false, time: xKind === "time" }] : []),
    ...series.map((s) => ({
      key: s.value,
      label: s.label ?? s.value,
      numeric: s.kind !== "time",
      time: s.kind === "time",
    })),
  ];

  return (
    <div className="h-full overflow-auto">
      <table className="w-full text-sm">
        <thead className="sticky top-0 bg-card/80 backdrop-blur">
          <tr className="text-left text-muted-foreground">
            {columns.map((c) => (
              <th key={c.key} className="px-2 py-1.5 font-medium">
                {c.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.points.map((row, i) => (
            <tr key={i} className="border-t border-border/60">
              {columns.map((c) => (
                <td
                  key={c.key}
                  className={`px-2 py-1.5 ${c.numeric ? "tabular text-right" : "text-foreground"}`}
                >
                  {formatCell(row[c.key], c.time ? dateTime : undefined)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function formatCell(
  v: string | number | null | undefined,
  formatTime?: (input: string | number | Date) => string,
): string {
  if (v == null) return "—";
  // Time columns route through the region/preference formatter; a
  // value that won't parse falls back to its raw string rather than
  // throwing or showing "Invalid Date".
  if (formatTime && (typeof v === "string" || typeof v === "number")) {
    try {
      return formatTime(v);
    } catch {
      return String(v);
    }
  }
  return String(v);
}
