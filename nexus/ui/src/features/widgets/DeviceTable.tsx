import type { Widget, WidgetData } from "@/data/types";
import { Empty } from "@/features/state/Empty";

// Device / row table. Columns come from the field mapping: the `x`
// column (if any) is the leading key column, followed by each mapped
// series. Renders the raw query rows — no client-side aggregation, no
// fabricated rows (F0/F6). Numeric series use tabular figures.
export function DeviceTable({
  widget,
  data,
}: {
  widget: Widget;
  data: WidgetData;
}) {
  const { x, series } = widget.config.fields;
  if (data.points.length === 0) {
    return <Empty title={widget.title} description="No rows" />;
  }

  const columns = [
    ...(x ? [{ key: x, label: x, numeric: false }] : []),
    ...series.map((s) => ({
      key: s.value,
      label: s.label ?? s.value,
      numeric: true,
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
                  {formatCell(row[c.key])}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function formatCell(v: string | number | null | undefined): string {
  if (v == null) return "—";
  return String(v);
}
