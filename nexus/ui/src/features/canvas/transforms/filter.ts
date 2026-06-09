import type { SeriesPoint } from "@/data/types";

type Op = "=" | "!=" | ">" | ">=" | "<" | "<=";

// Keep only the rows where `field <op> value` holds. Equality compares as
// strings (so it works for text columns); the ordering operators compare
// numerically and drop rows whose field isn't numeric. Pure — returns a
// new array (F6).
export function applyFilter(
  rows: ReadonlyArray<SeriesPoint>,
  field: string,
  op: Op,
  value: string,
): SeriesPoint[] {
  if (!field) return [...rows];
  return rows.filter((row) => keep(row[field], op, value));
}

function keep(cell: SeriesPoint[string], op: Op, value: string): boolean {
  if (op === "=" || op === "!=") {
    const eq = String(cell ?? "") === value;
    return op === "=" ? eq : !eq;
  }
  const a = typeof cell === "number" ? cell : Number(cell);
  const b = Number(value);
  if (!Number.isFinite(a) || !Number.isFinite(b)) return false;
  switch (op) {
    case ">":
      return a > b;
    case ">=":
      return a >= b;
    case "<":
      return a < b;
    case "<=":
      return a <= b;
  }
}
