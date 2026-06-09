import type { QueryResponse } from "@/api/types";
import { Empty } from "@/features/state/Empty";

// Renders a `POST /query` result as a table: the column schema as the
// header, the raw JSON rows as cells. Pure presentation — it shows
// exactly what the server returned, nothing synthesised (F0). Numeric and
// timestamp columns use tabular figures so they align.
const NUMERIC = new Set(["int", "float"]);

export function ResultGrid({ result }: { result: QueryResponse }) {
  if (result.rows.length === 0) {
    return <Empty title="No rows" description="The query returned nothing." />;
  }
  return (
    <div className="scrollbar-thin h-full overflow-auto rounded-lg border border-border/60">
      <table className="w-full text-sm">
        <thead className="sticky top-0 bg-card/90 backdrop-blur">
          <tr className="text-left">
            {result.columns.map((c) => (
              <th key={c.name} className="px-3 py-2 font-medium">
                <span className="text-foreground">{c.name}</span>
                <span className="ms-2 text-xs text-muted-foreground">{c.type}</span>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {result.rows.map((row, i) => (
            <tr key={i} className="border-t border-border/50">
              {result.columns.map((c) => {
                const v = (row as Record<string, unknown>)[c.name];
                return (
                  <td
                    key={c.name}
                    className={`px-3 py-1.5 ${NUMERIC.has(c.type) || c.type === "timestamp" ? "tabular" : "text-foreground"}`}
                  >
                    {v == null ? "—" : String(v)}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
