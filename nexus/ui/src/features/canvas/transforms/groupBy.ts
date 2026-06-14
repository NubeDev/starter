import type { SeriesPoint } from "@/data/types";

type Agg = "sum" | "avg" | "min" | "max" | "count";

// Collapse rows into one row per distinct `by` value, aggregating
// `field`'s numeric values into a new `as` column. Group order follows
// first appearance so the output is deterministic. `count` ignores the
// field's values (it counts rows); the others skip non-numeric cells.
// Pure (F6).
export function applyGroupBy(
  rows: ReadonlyArray<SeriesPoint>,
  by: string,
  field: string,
  agg: Agg,
  as: string,
): SeriesPoint[] {
  if (!by || !as) return [...rows];
  const order: string[] = [];
  const buckets = new Map<string, number[]>();
  const keyValue = new Map<string, SeriesPoint[string]>();

  for (const row of rows) {
    const raw = row[by];
    const key = String(raw ?? "");
    if (!buckets.has(key)) {
      buckets.set(key, []);
      keyValue.set(key, raw);
      order.push(key);
    }
    const n = numeric(row[field]);
    if (n != null) buckets.get(key)!.push(n);
  }

  return order.map((key) => ({
    [by]: keyValue.get(key) ?? key,
    [as]: aggregate(buckets.get(key)!, agg),
  }));
}

function aggregate(values: number[], agg: Agg): number {
  if (agg === "count") return values.length;
  if (values.length === 0) return 0;
  switch (agg) {
    case "sum":
      return values.reduce((a, b) => a + b, 0);
    case "avg":
      return values.reduce((a, b) => a + b, 0) / values.length;
    case "min":
      return Math.min(...values);
    case "max":
      return Math.max(...values);
  }
}

function numeric(v: SeriesPoint[string]): number | null {
  if (typeof v === "number") return Number.isFinite(v) ? v : null;
  if (typeof v === "string" && v.trim() !== "") {
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  }
  return null;
}
