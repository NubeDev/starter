import type { SeriesPoint } from "@/data/types";

type Calc = "last" | "first" | "sum" | "avg" | "min" | "max" | "count";

// Reduce every row to a single row holding one calculated value of
// `field` under the `as` column — the path stat/gauge panels take to
// collapse a series to one number. `last`/`first` return the raw cell (so
// a text column survives); the numeric calcs skip non-numeric cells. An
// empty input yields a single row with `null`. Pure (F6).
export function applyReduce(
  rows: ReadonlyArray<SeriesPoint>,
  field: string,
  calc: Calc,
  as: string,
): SeriesPoint[] {
  if (!field || !as) return [...rows];

  if (calc === "first" || calc === "last") {
    const row = calc === "first" ? rows[0] : rows[rows.length - 1];
    return [{ [as]: row ? (row[field] ?? null) : null }];
  }

  const values = rows
    .map((r) => numeric(r[field]))
    .filter((n): n is number => n != null);

  if (calc === "count") return [{ [as]: rows.length }];
  if (values.length === 0) return [{ [as]: null }];

  let out: number;
  switch (calc) {
    case "sum":
      out = values.reduce((a, b) => a + b, 0);
      break;
    case "avg":
      out = values.reduce((a, b) => a + b, 0) / values.length;
      break;
    case "min":
      out = Math.min(...values);
      break;
    case "max":
      out = Math.max(...values);
      break;
  }
  return [{ [as]: out }];
}

function numeric(v: SeriesPoint[string]): number | null {
  if (typeof v === "number") return Number.isFinite(v) ? v : null;
  if (typeof v === "string" && v.trim() !== "") {
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  }
  return null;
}
