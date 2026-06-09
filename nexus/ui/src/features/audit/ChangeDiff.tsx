import { useMemo } from "react";

// A compact before -> after diff for one change's snapshots. Shows only the
// keys whose values differ (an update touches a few fields; dumping every field
// would bury the change). A create has no `before`, a delete no `after`; those
// render as the whole added/removed object.
export function ChangeDiff({
  before,
  after,
}: {
  before?: Record<string, unknown> | null;
  after?: Record<string, unknown> | null;
}) {
  const rows = useMemo(() => diffRows(before ?? null, after ?? null), [before, after]);

  if (rows.length === 0) {
    return (
      <p className="text-xs text-muted-foreground">
        No field-level changes recorded.
      </p>
    );
  }

  return (
    <table className="w-full text-left text-xs">
      <thead>
        <tr className="text-muted-foreground">
          <th className="py-1 pr-3 font-medium">Field</th>
          <th className="py-1 pr-3 font-medium">Before</th>
          <th className="py-1 font-medium">After</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.key} className="border-t border-border/50 align-top">
            <td className="py-1 pr-3 font-mono">{row.key}</td>
            <td className="py-1 pr-3 font-mono text-destructive">{row.before}</td>
            <td className="py-1 font-mono text-emerald-600 dark:text-emerald-400">
              {row.after}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

interface DiffRow {
  key: string;
  before: string;
  after: string;
}

// The union of keys present in either snapshot, keeping only those whose
// rendered value changed. Values are rendered as compact JSON so nested objects
// stay readable on one line.
function diffRows(
  before: Record<string, unknown> | null,
  after: Record<string, unknown> | null,
): DiffRow[] {
  const keys = new Set<string>([
    ...Object.keys(before ?? {}),
    ...Object.keys(after ?? {}),
  ]);
  const rows: DiffRow[] = [];
  for (const key of keys) {
    const b = render(before?.[key]);
    const a = render(after?.[key]);
    if (b !== a) rows.push({ key, before: b, after: a });
  }
  return rows.sort((x, y) => x.key.localeCompare(y.key));
}

function render(value: unknown): string {
  if (value === undefined) return "—";
  if (value === null) return "null";
  if (typeof value === "string") return value;
  return JSON.stringify(value);
}
